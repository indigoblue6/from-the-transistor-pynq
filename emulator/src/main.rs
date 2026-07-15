//! 参照エミュレータのコマンドライン入口。

use std::{env, fs, io::{self, Write}, process::ExitCode};
use pynq_cpu_emulator::{Cpu, StopReason};

fn usage() {
    eprintln!("使い方: pynq-cpu-emulator PROGRAM [--trace] [--max-steps N] [--dump-registers]");
}

fn run() -> Result<u8, String> {
    let mut args = env::args().skip(1);
    let program = args.next().ok_or_else(|| "プログラムファイルが指定されていません".to_string())?;
    let mut trace = false; let mut dump = false; let mut max_steps = 100_000u64;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--trace" => trace = true,
            "--dump-registers" => dump = true,
            "--max-steps" => max_steps = args.next().ok_or("--max-stepsには値が必要です")?.parse().map_err(|_| "ステップ数が不正です")?,
            _ => return Err(format!("不明なオプション: {arg}")),
        }
    }
    let image = fs::read(&program).map_err(|e| format!("{program}を読めません: {e}"))?;
    let mut cpu = Cpu::new(&image, trace).map_err(|e| e.to_string())?;
    let result = cpu.run(max_steps);
    io::stdout().write_all(&cpu.uart_output).map_err(|e| e.to_string())?;
    io::stdout().flush().map_err(|e| e.to_string())?;
    if dump { eprint!("{}", cpu.register_dump()); }
    match result.map_err(|e| e.to_string())? {
        StopReason::Halt | StopReason::SimExit(0) => Ok(0),
        StopReason::SimExit(code) => { eprintln!("SIM_EXIT失敗: {code}"); Ok(1) }
    }
}

fn main() -> ExitCode {
    match run() { Ok(code) => ExitCode::from(code), Err(error) => { usage(); eprintln!("エラー: {error}"); ExitCode::FAILURE } }
}
