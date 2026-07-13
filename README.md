# aws-util

A small Rust CLI that starts or stops a single, pre-configured AWS EC2
instance and waits for it to reach the desired state — a typed,
interactive replacement for a handful of `aws ec2` commands you'd
otherwise run by hand:

```cmd
set PROFILE=my-profile
set INSTANCE_ID=i-0123456789abcdef0

aws ec2 start-instances --instance-ids %INSTANCE_ID% --profile %PROFILE% --no-cli-pager
aws ec2 wait instance-running --instance-ids %INSTANCE_ID% --profile %PROFILE%
```

`aws-util` remembers your profile and instance ID in a small config file
next to the executable, so day-to-day usage is just:

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
- An AWS CLI profile already configured (`aws configure --profile
  <name>`, or SSO) with permission to describe/start/stop the target
  EC2 instance.

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
| `--configure` | Runs the interactive setup: type in a profile name, fuzzy-search/select an instance, and save both to the config file. Only configures — does not start or stop anything. Conflicts with `--stop`/`--schedule-shutdown`. |
| `--schedule-shutdown [HH:MM]` | Tells the **instance itself** to shut down at the given 24-hour local clock time (default `18:30` if omitted; rolls over to tomorrow if that time already passed today), via an SSM Run Command. First checks whether a shutdown is already pending and leaves it alone if so, instead of scheduling a duplicate. Conflicts with `--configure`/`--start`/`--stop`. See [Auto-shutdown](#auto-shutdown-via-ssm) below. |

If no config file exists yet (or it's missing required fields), running
`aws-util`, `aws-util --start`, `aws-util --stop`, or
`aws-util --schedule-shutdown` will print:

```
Partial or no config found. Run `aws-util --configure` first.
```

and exit without prompting — run `aws-util --configure` to set it up.

### First-time setup

```sh
aws-util --configure
```

You'll be asked to:
1. Type in an AWS profile name (must already exist in your AWS CLI
   config).
2. Fuzzy-search and select an EC2 instance from the list of instances
   visible to that profile.

Both are saved to the config file. Re-run `--configure` any time to
change either value.

## Configuration

`aws-util` reads/writes a JSON config file named `aws_util_conf.json`:

- **Release builds:** next to the executable (same directory as
  `aws-util.exe`).
- **Debug builds (`cargo run`):** in the current working directory, for
  convenience during development.

```json
{
  "profile": "my-profile",
  "instance_id": "i-0123456789abcdef0"
}
```

Both fields are optional in the file itself — if the file is missing a
field (e.g. from an older version of this tool, or partial manual
editing), `aws-util --configure` will only need to fill in what's
missing on the next reconfigure; a genuinely corrupt/unparseable file is
backed up to `aws_util_conf.json.bak` with a warning instead of crashing.

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

