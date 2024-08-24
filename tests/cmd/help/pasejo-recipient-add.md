```console
$ pasejo recipient help add
? 101
thread 'main' panicked at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/builder/debug_asserts.rs:112:17:
Command add: Short option names must be unique for each argument, but '-p' is in use by both 'public_key' and 'path'
stack backtrace:
   0: rust_begin_unwind
             at /rustc/b5fd9f6f1061b79c045cc08fe03e00caad536800/library/std/src/panicking.rs:665:5
   1: core::panicking::panic_fmt
             at /rustc/b5fd9f6f1061b79c045cc08fe03e00caad536800/library/core/src/panicking.rs:74:14
   2: clap_builder::builder::debug_asserts::assert_app
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/builder/debug_asserts.rs:112:17
   3: clap_builder::builder::command::Command::_build_self
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/builder/command.rs:4133:13
   4: clap_builder::builder::command::Command::_build_subcommand
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/builder/command.rs:4220:9
   5: clap_builder::parser::parser::Parser::parse_help_subcommand
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/parser/parser.rs:642:21
   6: clap_builder::parser::parser::Parser::get_matches_with
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/parser/parser.rs:94:33
   7: clap_builder::parser::parser::Parser::parse_subcommand
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/parser/parser.rs:720:37
   8: clap_builder::parser::parser::Parser::get_matches_with
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/parser/parser.rs:474:17
   9: clap_builder::builder::command::Command::_do_parse
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/builder/command.rs:4010:29
  10: clap_builder::builder::command::Command::try_get_matches_from_mut
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/builder/command.rs:830:9
  11: clap_builder::builder::command::Command::get_matches_from
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/builder/command.rs:701:9
  12: clap_builder::builder::command::Command::get_matches
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/builder/command.rs:610:9
  13: clap_builder::derive::Parser::parse
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/derive.rs:27:27
  14: pasejo::main
             at ./src/main.rs:17:15
  15: core::ops::function::FnOnce::call_once
             at /rustc/b5fd9f6f1061b79c045cc08fe03e00caad536800/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

```

```console
$ pasejo recipient add --help
? 101
thread 'main' panicked at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/builder/debug_asserts.rs:112:17:
Command add: Short option names must be unique for each argument, but '-p' is in use by both 'public_key' and 'path'
stack backtrace:
   0: rust_begin_unwind
             at /rustc/b5fd9f6f1061b79c045cc08fe03e00caad536800/library/std/src/panicking.rs:665:5
   1: core::panicking::panic_fmt
             at /rustc/b5fd9f6f1061b79c045cc08fe03e00caad536800/library/core/src/panicking.rs:74:14
   2: clap_builder::builder::debug_asserts::assert_app
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/builder/debug_asserts.rs:112:17
   3: clap_builder::builder::command::Command::_build_self
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/builder/command.rs:4133:13
   4: clap_builder::builder::command::Command::_build_subcommand
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/builder/command.rs:4220:9
   5: clap_builder::parser::parser::Parser::parse_subcommand
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/parser/parser.rs:703:27
   6: clap_builder::parser::parser::Parser::get_matches_with
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/parser/parser.rs:474:17
   7: clap_builder::parser::parser::Parser::parse_subcommand
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/parser/parser.rs:720:37
   8: clap_builder::parser::parser::Parser::get_matches_with
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/parser/parser.rs:474:17
   9: clap_builder::builder::command::Command::_do_parse
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/builder/command.rs:4010:29
  10: clap_builder::builder::command::Command::try_get_matches_from_mut
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/builder/command.rs:830:9
  11: clap_builder::builder::command::Command::get_matches_from
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/builder/command.rs:701:9
  12: clap_builder::builder::command::Command::get_matches
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/builder/command.rs:610:9
  13: clap_builder::derive::Parser::parse
             at /home/seb/.local/share/cargo/registry/src/index.crates.io-6f17d22bba15001f/clap_builder-4.5.7/src/derive.rs:27:27
  14: pasejo::main
             at ./src/main.rs:17:15
  15: core::ops::function::FnOnce::call_once
             at /rustc/b5fd9f6f1061b79c045cc08fe03e00caad536800/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

```