#!/bin/sh
set -eu

check_error() {
    name=$1
    expected=$2
    if cargo run --quiet --manifest-path compiler/Cargo.toml -- \
        "examples/c/negative/$name.pc" --check >build/negative.out 2>build/negative.err; then
        echo "$nameが誤ってコンパイル成功しました" >&2
        exit 1
    fi
    if ! grep -q "$expected" build/negative.err; then
        echo "$nameの診断に期待文字列「$expected」がありません" >&2
        cat build/negative.err >&2
        exit 1
    fi
}

mkdir -p build
check_error undefined_variable '未定義変数'
check_error duplicate_variable '重複定義'
check_error duplicate_function '重複定義'
check_error wrong_argument_count '引数は1個必要'
check_error wrong_argument_type '引数型'
check_error invalid_return 'void関数から値'
check_error assign_to_literal '左辺値'
check_error dereference_integer '非ポインタ'
check_error invalid_pointer_addition '適用できません'
check_error break_outside_loop 'ループ内'
check_error continue_outside_loop 'ループ内'
check_error unterminated_string '文字列リテラル'
check_error unterminated_comment 'ブロックコメント'
check_error syntax_error '型名'
check_error unsupported_feature '型名'

echo "PynqCネガティブテスト成功"
