# aws-util

A small Rust CLI that starts or stops one or more pre-configured AWS EC2
instances and waits for them to reach the desired state — a typed,
interactive replacement for a handful of `aws ec2` commands you'd
otherwise run by hand:

```cmd
set PROFILE=my-profile
set INSTANCE_ID=i-0123456789abcdef0

aws ec2 start-instances --instance-ids %INSTANCE_ID% --profile %PROFILE% --no-cli-pager
aws ec2 wait instance-running --instance-ids %INSTANCE_ID% --profile %PROFILE%
```

`aws-util` remembers one or more profile+instance pairs in a small config
file next to the executable, so day-to-day usage is just:

```sh
aws-util
```

## How it works

`aws-util` does **not** use the AWS SDK. It shells out to the `aws` CLI
(`aws ec2 describe-instances`, `start-instances`, `stop-instances`, and
`wait instance-running`/`instance-stopped`) and parses the JSON output.
This means:

- It reuses whatever `aws configure` / SSO setup you already have —
  nothing extra to authenticate.
- The compiled binary is small (~500 KB) and builds in seconds, since it
  doesn't statically link the AWS SDK, TLS stack, or an async runtime.

## Requirements

- The [AWS CLI](https://aws.amazon.com/cli/) (v2 recommended) installed
  and available on `PATH`.
- One or more AWS CLI profiles already configured (`aws configure
  --profile <name>`, or SSO) with permission to describe/start/stop the
  target EC2 instance(s).

## Installation / build

```sh
cargo build --release
```

The resulting binary is at `target/release/aws-util.exe` (Windows) or
`target/release/aws-util` (Linux/macOS). Copy it wherever you like — the
config file lives next to it (see [Configuration](#configuration)).

## Usage

```
aws-util [--configure] [--start | --stop | --schedule-shutdown [HH:MM]]
```

| Flag          | Behavior                                                                                                   |
| ------------- | ------------------------------------------------------------------------------------------------------------ |
| *(none)*      | Same as `--start`. This is the default.                                                                     |
| `--start`     | Explicit alias for the default: starts the configured instance and waits for `running` (no-op if already running). |
| `--stop`      | Stops the configured instance and waits for `stopped` (no-op if already stopped). Conflicts with `--start`/`--schedule-shutdown`. |
| `--configure` | Runs the interactive setup: add a new profile/instance pair, or pick an existing one to replace, then fuzzy-search/select an AWS CLI profile and an instance. Only configures — does not start or stop anything. Conflicts with `--stop`/`--schedule-shutdown`. |
| `--schedule-shutdown [HH:MM]` | Tells the **instance itself** to shut down at the given 24-hour local clock time (default `18:30` if omitted; rolls over to tomorrow if that time already passed today), via an SSM Run Command. First checks whether a shutdown is already pending and leaves it alone if so, instead of scheduling a duplicate. Conflicts with `--configure`/`--start`/`--stop`. See [Auto-shutdown](#auto-shutdown-via-ssm) below. |

If no profile/instance pair is configured yet, running `aws-util`,
`aws-util --start`, `aws-util --stop`, or `aws-util --schedule-shutdown`
will print:

```
No configuration found. Run `aws-util --configure` first.
```

and exit without prompting — run `aws-util --configure` to set it up.

### Multiple profiles/instances

`aws-util` can manage more than one EC2 instance (each under its own AWS
CLI profile, or even multiple instances under the same profile):

- If exactly **one** profile/instance pair is configured, it's used
  automatically — no prompt.
- If **more than one** is configured, `--start`/`--stop`/
  `--schedule-shutdown` show a fuzzy-search prompt to pick which one to
  act on for that invocation.
- `--configure` first lets you pick an existing pair to replace, or
  choose "+ Add new profile/instance" to add another one alongside the
  existing ones.

### First-time setup

```sh
aws-util --configure
```

If any profile/instance pairs are already configured, you'll first be
asked to pick one to replace, or "+ Add new profile/instance". Either
way, you'll then be asked to:
1. Fuzzy-search and select an AWS CLI profile (from `aws configure
   list-profiles` — fails with a clear error if none are configured yet).
2. Fuzzy-search and select an EC2 instance from the list of instances
   visible to that profile.

The result is saved to the config file. Re-run `--configure` any time to
add another pair or change an existing one.

## Configuration

`aws-util` reads/writes a JSON config file named `aws_util_conf.json`:

- **Release builds:** next to the executable (same directory as
  `aws-util.exe`).
- **Debug builds (`cargo run`):** in the current working directory, for
  convenience during development.

```json
[
  { "profile": "my-profile", "instance_id": "i-0123456789abcdef0" },
  { "profile": "other-profile", "instance_id": "i-0fedcba9876543210" }
]
```

A genuinely corrupt/unparseable file is backed up to
`aws_util_conf.json.bak` with a warning instead of crashing, and treated
as if no config existed.

The config file is git-ignored (see `.gitignore`) since it's
machine/user-specific.


## Auto-shutdown via SSM

Rather than running a background service on your machine to watch for
shutdown/logoff (complex, and platform-specific), `aws-util` can tell
the **EC2 instance itself** to shut down at a given local clock time:

```sh
aws-util --schedule-shutdown          # shuts down at 18:30 (today, or tomorrow if already past)
aws-util --schedule-shutdown 20:00    # shuts down at 20:00 (today, or tomorrow if already past)
```

`aws-util` computes the delay from your machine's current local time to
the next occurrence of that clock time, then sends a small shell script
to the instance via [AWS Systems Manager
Run Command](https://docs.aws.amazon.com/systems-manager/latest/userguide/execute-remote-commands.html)
(`AWS-RunShellScript`) that:
1. Checks whether a shutdown is already scheduled (via `shutdown --show`'s
   exit code) and leaves it alone if so — safe to run repeatedly without
   stacking multiple shutdowns.
2. Otherwise runs `shutdown -h +<minutes>` on the instance.

Because the instance's own OS timer does the work, the shutdown still
happens even if this tool (or your machine) isn't running when the delay
elapses.

**Requirements:** the instance must have the SSM Agent running and an
instance profile with SSM permissions (e.g.
`AmazonSSMManagedInstanceCore`) attached — if not, the command fails
with a clear error. Currently Linux/systemd only.

## Interrupting (Ctrl+C)

`aws-util` installs a Ctrl+C handler that restores the terminal cursor
(in case it was hidden by an interactive prompt) and exits cleanly with
code 130, instead of leaving the terminal in a bad state.

## Development

- `cargo build` — debug build.
- `cargo build --release` — optimized build (size-optimized release
  profile: `opt-level = "z"`, LTO, `panic = "abort"`, stripped symbols).
- VS Code debug configs are provided in `.vscode/launch.json` /
  `.vscode/tasks.json` (uses the Visual Studio Windows Debugger,
  `cppvsdbg`).

