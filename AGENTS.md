# AGENTS.md

## Build & deploy

- `go build -ldflags "-H=windowsgui" -o shutd.exe ./cmd/shutd` only compiles a
  binary to the given output path. It does **not** update the deployed binary.
- `go install -ldflags "-H=windowsgui" ./cmd/shutd` builds **and** installs to
  `%GOPATH%\bin\shutd.exe`.
- `%GOPATH%\bin\shutd.exe` is the deployment target: the startup `shutd.vbs`
  (see README) launches the app from there.
- When asked to "build it" (or equivalent), treat that as build **and**
  install — a repo-root build alone leaves the startup binary stale.
