use pynqc::{codegen::generate, lexer::lex, parser::parse, semantic::analyze};

#[test]
fn フレーム分岐呼出しを含むコードを生成する() {
    let source = "int add(int a,int b){return a+b;} int main(){int x=add(40,2); if(x==42)return 0; return 1;}";
    let mut program = parse(lex(source).unwrap()).unwrap();
    let info = analyze(&mut program).unwrap();
    let assembly = generate(&mut program, &info, false).unwrap();
    assert!(assembly.contains("_start:"));
    assert!(assembly.contains("call add"));
    assert!(assembly.contains("store r14, [r15 + 4]"));
}

#[test]
fn sizeof配列は全体サイズを生成する() {
    let source = "int main(){int values[3]; return sizeof(values);}";
    let mut program = parse(lex(source).unwrap()).unwrap();
    let info = analyze(&mut program).unwrap();
    let assembly = generate(&mut program, &info, false).unwrap();
    assert!(assembly.contains("movi r6, 12"));
}
