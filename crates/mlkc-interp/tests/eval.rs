//! What the interpreter computes: the meaning of the MIR the front end builds.
//!
//! Every test runs a function in both forms of MIR and checks that the two agree: a disagreement
//! is a bug of the construction of SSA, and a wrong word is a bug of the meaning itself.

mod harness;

use harness::{Compiled, NoHost, Output};
use mlkc_interp::{Trap, Value};

/// A program of one module, compiled for the interpreter.
fn program(source: &str) -> Compiled {
    harness::module(source)
}

#[test]
fn arithmetic_computes_over_immediates() {
    let program = program(
        "\
fun double(value: Int): Int =
    value + value

fun mixed(left: Int, right: Int): Int =
    left * right - left / right + (left - right)

fun negated(value: Int): Int =
    -value
",
    );

    assert_eq!(
        program.run("app::main::double", &[Value::Int(21)]),
        Ok(Value::Int(42)),
    );
    assert_eq!(
        program.run("app::main::double", &[Value::Int(-21)]),
        Ok(Value::Int(-42)),
    );
    assert_eq!(
        program.run("app::main::mixed", &[Value::Int(6), Value::Int(2)]),
        Ok(Value::Int(13)),
    );
    assert_eq!(
        program.run("app::main::mixed", &[Value::Int(7), Value::Int(3)]),
        Ok(Value::Int(23)),
    );
    assert_eq!(
        program.run("app::main::negated", &[Value::Int(5)]),
        Ok(Value::Int(-5)),
    );
}

#[test]
fn arithmetic_wraps_where_the_word_wraps() {
    // An immediate is 31 bits wide ([ADR-0018]): the sum of two large immediates is narrowed
    // the way the back end narrows it, so the interpreter and the module agree.
    //
    // [ADR-0018]: ../../docs/adr/0018-values-as-words.md
    let program = program(
        "\
fun sum(): Int =
    1073741823 + 1

fun difference(): Int =
    -1073741823 - 1073741823
",
    );

    assert_eq!(
        program.run("app::main::sum", &[]),
        Ok(Value::Int(-1_073_741_824))
    );
    assert_eq!(program.run("app::main::difference", &[]), Ok(Value::Int(2)));
}

#[test]
fn comparisons_and_booleans_compute() {
    let program = program(
        "\
fun ordered(left: Int, right: Int): Bool =
    left < right && left <= right || left > right || left >= right

fun different(left: Int, right: Int): Bool =
    left != right

fun both(): Bool =
    true && false
",
    );

    assert_eq!(
        program.run("app::main::ordered", &[Value::Int(1), Value::Int(2)]),
        Ok(Value::Bool(true)),
    );
    assert_eq!(
        program.run("app::main::different", &[Value::Int(1), Value::Int(2)]),
        Ok(Value::Bool(true)),
    );
    assert_eq!(
        program.run("app::main::different", &[Value::Int(2), Value::Int(2)]),
        Ok(Value::Bool(false)),
    );
    assert_eq!(program.run("app::main::both", &[]), Ok(Value::Bool(false)));
}

#[test]
fn a_choice_selects_the_arm_it_should() {
    let program = program(
        "\
fun pick(flag: Bool): Int =
    if flag then 1 else 2

fun ordered(low: Bool, high: Bool): Int =
    if low then 1 elif high then 2 else 3

fun where_a_value_belongs(flag: Bool): Int =
    (if flag then 1 else 2) + 3

fun through_a_let(flag: Bool): Int =
    let carried = if flag then 1 else 2 in
    carried + 3
",
    );

    assert_eq!(
        program.run("app::main::pick", &[Value::Bool(true)]),
        Ok(Value::Int(1)),
    );
    assert_eq!(
        program.run("app::main::pick", &[Value::Bool(false)]),
        Ok(Value::Int(2)),
    );
    assert_eq!(
        program.run("app::main::ordered", &[
            Value::Bool(false),
            Value::Bool(true)
        ]),
        Ok(Value::Int(2)),
    );
    assert_eq!(
        program.run("app::main::ordered", &[
            Value::Bool(false),
            Value::Bool(false)
        ]),
        Ok(Value::Int(3)),
    );
    assert_eq!(
        program.run("app::main::where_a_value_belongs", &[Value::Bool(false)]),
        Ok(Value::Int(5)),
    );
    assert_eq!(
        program.run("app::main::through_a_let", &[Value::Bool(true)]),
        Ok(Value::Int(4)),
    );
}

#[test]
fn recursion_descends_and_returns() {
    let program = program(
        "\
fun fib(value: Int): Int =
    if value < 2 then value else fib(value - 1) + fib(value - 2)
",
    );

    assert_eq!(
        program.run("app::main::fib", &[Value::Int(10)]),
        Ok(Value::Int(55)),
    );
}

#[test]
fn a_host_reads_what_the_program_prints() {
    let program = program(
        "\
#[extern]
fun print-int(value: Int): Unit

#[extern]
fun print-bool(value: Bool): Unit

fun report(value: Int): Unit =
    print-int(value + 1)

fun report_a_flag(flag: Bool): Unit =
    print-bool(flag)
",
    );

    let mut cfg = Output::default();
    program
        .cfg("app::main::report", &[Value::Int(7)], &mut cfg)
        .expect("the CFG form to run");

    let mut ssa = Output::default();
    program
        .ssa("app::main::report", &[Value::Int(7)], &mut ssa)
        .expect("the SSA form to run");

    assert_eq!(cfg.printed, ["8"]);
    assert_eq!(cfg.printed, ssa.printed);

    let mut cfg_flag = Output::default();
    program
        .cfg(
            "app::main::report_a_flag",
            &[Value::Bool(true)],
            &mut cfg_flag,
        )
        .expect("the CFG form to run");

    let mut ssa_flag = Output::default();
    program
        .ssa(
            "app::main::report_a_flag",
            &[Value::Bool(false)],
            &mut ssa_flag,
        )
        .expect("the SSA form to run");

    assert_eq!(cfg_flag.printed, ["true"]);
    assert_eq!(ssa_flag.printed, ["false"]);
}

#[test]
fn a_run_without_a_host_refuses_an_external_call() {
    let program = program(
        "\
#[extern]
fun print-int(value: Int): Unit

fun report(value: Int): Unit =
    print-int(value)
",
    );

    let refused = program
        .run("app::main::report", &[Value::Int(7)])
        .expect_err("a run without a host to refuse the call");

    assert!(matches!(refused, Trap::Host { .. }), "{refused}");
}

#[test]
fn division_by_zero_is_a_trap() {
    let program = program(
        "\
fun divide(left: Int, right: Int): Int =
    left / right
",
    );

    let trapped = program
        .ssa(
            "app::main::divide",
            &[Value::Int(1), Value::Int(0)],
            &mut NoHost,
        )
        .expect_err("the division to trap");

    assert!(matches!(trapped, Trap::DivideByZero { .. }), "{trapped}");
}

#[test]
fn a_string_constant_is_not_run_yet() {
    let program = program(
        "\
fun text(): String =
    \"hello\"
",
    );

    let trapped = program
        .ssa("app::main::text", &[], &mut NoHost)
        .expect_err("the string constant to be reported");

    assert!(
        matches!(trapped, Trap::Unsupported { what, .. } if what == "a string constant"),
        "{trapped}",
    );
}
