# ferrite ⚡
> A fast, friendly HTTP client for the terminal — built in Rust

[![Crates.io](https://img.shields.io/crates/v/ferrite.svg)](https://crates.io/crates/ferrite)
[![CI](https://github.com/MFAIZAN20/ferrite/actions/workflows/ci.yml/badge.svg)](https://github.com/MFAIZAN20/ferrite/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

## Install

```bash
cargo install ferrite
```

## Quick Start

```bash
http GET https://httpbin.org/get
http POST https://httpbin.org/post name=faizan age:=22
http --form POST https://httpbin.org/post field=value
http -a user:pass https://httpbin.org/basic-auth/user/pass
http --auth-type bearer --auth "$TOKEN" https://api.example.com/me
http --session dev GET https://api.example.com/profile
http --download https://example.com/file.zip
http diff https://api.example.com/v1/user/42 https://api.example.com/v2/user/42
```

## Request Items

| Operator | Meaning | Example |
|---|---|---|
| `key:value` | Header | `Accept:application/json` |
| `key=value` | String field | `name=faizan` |
| `key:=value` | Raw JSON value | `age:=22` |
| `key==value` | Query parameter | `page==2` |
| `key@/path` | File upload | `avatar@/tmp/me.png` |
| `key@/path;type=mime` | File upload with MIME | `file@/tmp/a.bin;type=application/octet-stream` |
| `key=@/path` | Field from file | `payload=@/tmp/body.txt` |
| `key:=@/path` | JSON field from file | `config:=@/tmp/config.json` |

## Output Control

```bash
http GET https://httpbin.org/json --print=h
http GET https://httpbin.org/json --print=b --pretty=none
http GET https://httpbin.org/json --style=dracula
```

- `--print`: request/response sections (`H`, `B`, `h`, `b`)
- `--pretty`: `all`, `colors`, `format`, `none`
- `--style`: `monokai`, `solarized`, `dracula`, `autumn`

## Auth

```bash
http --auth user:pass https://api.example.com/basic
http --auth-type bearer --auth "$TOKEN" https://api.example.com/me
http --auth-type digest --auth user:pass https://api.example.com/digest
```

## Sessions

Named sessions persist headers, auth, and cookies:

```bash
http --session prod POST https://api.example.com/login username=faizan password=secret
http --session prod GET https://api.example.com/me
http --session prod --session-read-only GET https://api.example.com/me
```

## Environment Profiles

Profile file example: `~/.config/ferrite/envs/prod.json`

```json
{
  "base_url": "https://api.example.com",
  "headers": {
    "X-API-Version": "2"
  },
  "variables": {
    "USER_ID": "42"
  }
}
```

```bash
http run get-user --env-profile prod
```

## Request Collections

```bash
http save login -- POST https://api.example.com/login username=faizan password={PASSWORD}
http list
http run login --env-profile prod
http delete login
```

## AI Assistant

Set API key:

```bash
export FERRITE_AI_KEY=your_api_key
```

Then:

```bash
http ai "Create a POST request to https://api.example.com/users with name Faizan and role admin"
```

## Response Diffing

```bash
http diff https://api.example.com/v1/user/42 https://api.example.com/v2/user/42
```

## Download Mode

```bash
http --download https://example.com/archive.zip
http --download --continue --output archive.zip https://example.com/archive.zip
```

## Configuration

File: `~/.config/ferrite/config.json`

| Key | Type | Default | Description |
|---|---|---|---|
| `default_options` | `string[]` | `[]` | Default CLI flags prepended before explicit args |
| `default_scheme` | `string` | `https` | Scheme applied when URL has no scheme |
| `plugins_dir` | `string` | `~/.config/ferrite/plugins` | Plugin manifest directory |
| `output_theme` | `string` | `monokai` | Default theme |
| `pretty` | `string` | `all` | Default pretty mode |
| `verify` | `bool` | `true` | TLS cert verification |

## Comparison with HTTPie

| Feature              | HTTPie | ferrite |
|----------------------|--------|---------|
| JSON formatting      | ✅     | ✅      |
| Sessions             | ✅     | ✅      |
| Digest auth          | ✅     | ✅      |
| Env profiles         | ❌     | ✅      |
| Request collections  | ❌     | ✅      |
| AI assistant         | ❌     | ✅      |
| Response diffing     | ❌     | ✅      |
| Written in Rust      | ❌     | ✅      |
| Binary size          | ~15MB  | ~3MB    |

## Contributing

Issues and PRs are welcome. Please run:

```bash
cargo fmt --all
cargo clippy -- -D warnings
cargo test --all
```

## License

MIT. See [LICENSE](LICENSE).
