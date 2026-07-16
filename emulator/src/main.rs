//! 参照エミュレータのコマンドライン入口。

use pynq_cpu_emulator::{Cpu, StopReason};
use std::{
    env, fs,
    io::{self, Write},
    process::ExitCode,
};

fn usage() {
    eprintln!(
        "使い方: pynq-cpu-emulator PROGRAM [--trace] [--trace-interrupts] [--trace-syscalls] [--uart-input FILE] [--max-steps N] [--dump-registers] [--dump-csrs]"
    );
}

fn run() -> Result<u8, String> {
    let mut args = env::args().skip(1);
    let program = args
        .next()
        .ok_or_else(|| "プログラムファイルが指定されていません".to_string())?;
    let mut trace = false;
    let mut trace_interrupts = false;
    let mut trace_syscalls = false;
    let mut dump = false;
    let mut dump_csrs = false;
    let mut uart_input: Option<String> = None;
    let mut max_steps = 100_000u64;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--trace" => trace = true,
            "--trace-interrupts" => trace_interrupts = true,
            "--trace-syscalls" => trace_syscalls = true,
            "--dump-registers" => dump = true,
            "--dump-csrs" => dump_csrs = true,
            "--uart-input" => {
                uart_input = Some(args.next().ok_or("--uart-inputにはファイルが必要です")?)
            }
            "--max-steps" => {
                max_steps = args
                    .next()
                    .ok_or("--max-stepsには値が必要です")?
                    .parse()
                    .map_err(|_| "ステップ数が不正です")?
            }
            _ => return Err(format!("不明なオプション: {arg}")),
        }
    }
    let image = fs::read(&program).map_err(|e| format!("{program}を読めません: {e}"))?;
    let mut cpu = Cpu::new(&image, trace).map_err(|e| e.to_string())?;
    cpu.set_trace_interrupts(trace_interrupts);
    cpu.set_trace_syscalls(trace_syscalls);
    if let Some(path) = uart_input {
        let input = fs::read(&path).map_err(|e| format!("{path}を読めません: {e}"))?;
        cpu.queue_uart_input(&input);
    }
    let result = cpu.run(max_steps);
    io::stdout()
        .write_all(&cpu.uart_output)
        .map_err(|e| e.to_string())?;
    io::stdout().flush().map_err(|e| e.to_string())?;
    if dump {
        eprint!("{}", cpu.register_dump());
    }
    if dump_csrs {
        eprint!("{}", cpu.csr_dump());
    }
    match result.map_err(|e| e.to_string())? {
        StopReason::Halt | StopReason::SimExit(0) => Ok(0),
        StopReason::SimExit(code) => {
            eprintln!("SIM_EXIT失敗: {code}");
            Ok(1)
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            usage();
            eprintln!("エラー: {error}");
            ExitCode::FAILURE
        }
    }
}
