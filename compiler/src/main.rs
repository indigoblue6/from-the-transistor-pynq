//! `pynqc`コマンドライン入口。

use pynqc::{
    codegen::generate, diagnostic::Diagnostic, lexer::lex, parser::parse, semantic::analyze,
    source::Source,
};
use std::{env, fs, path::PathBuf, process::ExitCode};

#[derive(Default)]
struct Options {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    tokens: bool,
    ast: bool,
    typed_ast: bool,
    emit_asm: bool,
    check: bool,
    debug_codegen: bool,
}

fn usage() {
    eprintln!("使い方: pynqc INPUT.pc [-o OUTPUT.s] [--emit-tokens|--emit-ast|--emit-typed-ast|--emit-asm|--check] [--debug-codegen]");
}

fn options() -> Result<Options, String> {
    let mut result = Options::default();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" => result.output = Some(args.next().ok_or("-oには出力パスが必要です")?.into()),
            "--emit-tokens" => result.tokens = true,
            "--emit-ast" => result.ast = true,
            "--emit-typed-ast" => result.typed_ast = true,
            "--emit-asm" => result.emit_asm = true,
            "--check" => result.check = true,
            "--debug-codegen" => result.debug_codegen = true,
            value if value.starts_with('-') => return Err(format!("不明なオプション: {value}")),
            value if result.input.is_none() => result.input = Some(value.into()),
            value => return Err(format!("入力ファイルを複数指定できません: {value}")),
        }
    }
    if result.input.is_none() {
        return Err("入力ファイルがありません".into());
    }
    Ok(result)
}

fn run() -> Result<(), (Option<Source>, Diagnostic)> {
    let options = options().map_err(|message| {
        (
            None,
            Diagnostic::new(
                pynqc::diagnostic::DiagnosticKind::Tool,
                Default::default(),
                message,
            ),
        )
    })?;
    let path = options.input.unwrap();
    let name = path.display().to_string();
    let text = fs::read_to_string(&path).map_err(|error| {
        (
            None,
            Diagnostic::new(
                pynqc::diagnostic::DiagnosticKind::Tool,
                Default::default(),
                format!("{name}を読めません: {error}"),
            ),
        )
    })?;
    let source = Source::new(name, text);
    let tokens = lex(&source.text).map_err(|error| (Some(source.clone()), error))?;
    if options.tokens {
        println!("{tokens:#?}");
        return Ok(());
    }
    let mut program = parse(tokens).map_err(|error| (Some(source.clone()), error))?;
    if options.ast {
        println!("{program:#?}");
        return Ok(());
    }
    let info = analyze(&mut program).map_err(|error| (Some(source.clone()), error))?;
    if options.typed_ast {
        println!("{program:#?}");
        return Ok(());
    }
    if options.check {
        return Ok(());
    }
    let assembly = generate(&mut program, &info, options.debug_codegen)
        .map_err(|error| (Some(source.clone()), error))?;
    if options.emit_asm || options.output.is_none() {
        print!("{assembly}");
    }
    if let Some(output) = options.output {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                (
                    Some(source.clone()),
                    Diagnostic::new(
                        pynqc::diagnostic::DiagnosticKind::Tool,
                        Default::default(),
                        error.to_string(),
                    ),
                )
            })?;
        }
        fs::write(&output, assembly).map_err(|error| {
            (
                Some(source.clone()),
                Diagnostic::new(
                    pynqc::diagnostic::DiagnosticKind::Tool,
                    Default::default(),
                    format!("{}へ書けません: {error}", output.display()),
                ),
            )
        })?;
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err((source, error)) => {
            usage();
            if let Some(source) = source {
                eprintln!("{}", error.render(&source));
            } else {
                eprintln!("エラー: {error}");
            }
            ExitCode::FAILURE
        }
    }
}
