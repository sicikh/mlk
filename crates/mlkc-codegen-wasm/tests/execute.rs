//! Running what the back end emits: a module is instantiated, and the immediates it computes
//! are read back from the words it returns.

mod harness;

use harness::Compiled;
use wasmtime::{AnyRef, Config, Engine, I31, Instance, Module, Store, Val};

/// Instantiates the module of `compiled`, with the proposals it uses turned on.
fn instantiate(compiled: &Compiled) -> (Store<()>, Instance) {
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_function_references(true);

    let engine = Engine::new(&config).expect("the engine to start");
    let module = Module::new(&engine, &compiled.wasm.bytes).expect("the emitted module to compile");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("the module to instantiate");

    (store, instance)
}

/// Calls the exported function `name` with immediates, and reads the immediate it returns.
fn call(store: &mut Store<()>, instance: &Instance, name: &str, args: &[i32]) -> i32 {
    let function = instance
        .get_func(&mut *store, name)
        .unwrap_or_else(|| panic!("`{name}` to be exported from the module"));

    let arguments: Vec<Val> = args
        .iter()
        .map(|arg| {
            let value = I31::new_i32(*arg).expect("the argument to be a 31-bit immediate");

            Val::AnyRef(Some(AnyRef::from_i31(&mut *store, value)))
        })
        .collect();
    let mut results = [Val::AnyRef(None)];

    function
        .call(&mut *store, &arguments, &mut results)
        .unwrap_or_else(|error| panic!("`{name}` to run: {error}"));

    let result = results[0].unwrap_anyref().expect("a word to come back");
    let result = result.as_i31(&*store).expect("the result to be read");

    result.expect("an immediate to come back").get_i32()
}

#[test]
fn an_immediate_goes_in_and_comes_back_out() {
    let compiled = harness::module("pub fun double(value: Int): Int = value + value\n");

    compiled.validate();

    let (mut store, instance) = instantiate(&compiled);

    assert_eq!(call(&mut store, &instance, "double", &[21]), 42);
    assert_eq!(call(&mut store, &instance, "double", &[-21]), -42);
}

#[test]
fn a_branch_decides() {
    let compiled = harness::module(
        "\
pub fun pick(flag: Bool): Int =
    if flag then 1 else 2

pub fun ordered(low: Bool, high: Bool): Int =
    if low then 1 elif high then 2 else 3
",
    );

    compiled.validate();

    let (mut store, instance) = instantiate(&compiled);

    assert_eq!(call(&mut store, &instance, "pick", &[1]), 1);
    assert_eq!(call(&mut store, &instance, "pick", &[0]), 2);
    assert_eq!(call(&mut store, &instance, "ordered", &[1, 0]), 1);
    assert_eq!(call(&mut store, &instance, "ordered", &[0, 1]), 2);
    assert_eq!(call(&mut store, &instance, "ordered", &[0, 0]), 3);
}

#[test]
fn recursion_descends_and_returns() {
    let compiled = harness::module(
        "pub fun fib(value: Int): Int =\n    \
         if value < 2 then value else fib(value - 1) + fib(value - 2)\n",
    );

    compiled.validate();

    let (mut store, instance) = instantiate(&compiled);

    assert_eq!(call(&mut store, &instance, "fib", &[10]), 55);
}

#[test]
fn arithmetic_and_comparisons_compute() {
    let compiled = harness::module(
        "\
pub fun mixed(left: Int, right: Int): Int =
    left * right - left / right + (left - right)

pub fun different(left: Int, right: Int): Bool =
    left != right
",
    );

    compiled.validate();

    let (mut store, instance) = instantiate(&compiled);

    assert_eq!(
        call(&mut store, &instance, "mixed", &[6, 2]),
        6 * 2 - 6 / 2 + (6 - 2)
    );
    assert_eq!(
        call(&mut store, &instance, "mixed", &[7, 3]),
        7 * 3 - 7 / 3 + (7 - 3)
    );
    assert_eq!(call(&mut store, &instance, "different", &[1, 2]), 1);
    assert_eq!(call(&mut store, &instance, "different", &[2, 2]), 0);
}

#[test]
fn a_lambda_captures_and_is_called() {
    let compiled = harness::module(
        "pub fun main(): Int =\n    \
         let base = 40 in\n    \
         let add = fn(x: Int) -> x + base in\n    \
         add(2)\n",
    );

    compiled.validate();

    let (mut store, instance) = instantiate(&compiled);

    assert_eq!(call(&mut store, &instance, "main", &[]), 42);
}

#[test]
fn a_lambda_that_captures_nothing_is_called() {
    let compiled = harness::module(
        "pub fun main(): Int =\n    \
         let double = fn(x: Int) -> x + x in\n    \
         double(21)\n",
    );

    compiled.validate();

    let (mut store, instance) = instantiate(&compiled);

    assert_eq!(call(&mut store, &instance, "main", &[]), 42);
}

#[test]
fn a_generalized_lambda_is_called_at_two_types() {
    let compiled = harness::module(
        "pub fun main(): Int =\n    \
         let id = fn(x) -> x in\n    \
         let _ = id(true) in\n    \
         id(42)\n",
    );

    compiled.validate();

    let (mut store, instance) = instantiate(&compiled);

    assert_eq!(call(&mut store, &instance, "main", &[]), 42);
}

#[test]
fn a_function_declared_in_a_local_is_called() {
    let compiled = harness::module(
        "pub fun main(): Int =\n    \
         local fun double(x: Int): Int = x * 2 in\n    \
         double(21)\n",
    );

    compiled.validate();

    let (mut store, instance) = instantiate(&compiled);

    assert_eq!(call(&mut store, &instance, "main", &[]), 42);
}

#[test]
fn functions_declared_in_a_local_call_each_other() {
    let compiled = harness::module(
        "pub fun main(): Int =\n    \
         local\n        \
             fun odd(n: Int): Bool = if n == 0 then false else even(n - 1)\n        \
             fun even(n: Int): Bool = if n == 0 then true else odd(n - 1)\n    \
         in\n    \
         if even(10) then 42 else 0\n",
    );

    compiled.validate();

    let (mut store, instance) = instantiate(&compiled);

    assert_eq!(call(&mut store, &instance, "main", &[]), 42);
}

#[test]
fn a_lambda_written_in_a_local_is_lifted() {
    let compiled = harness::module(
        "pub fun main(): Int =\n    \
         local fun apply(x: Int): Int = (fn(y: Int) -> y + x)(1) in\n    \
         apply(41)\n",
    );

    compiled.validate();

    let (mut store, instance) = instantiate(&compiled);

    assert_eq!(call(&mut store, &instance, "main", &[]), 42);
}

#[test]
fn a_lambda_calls_a_function_declared_in_a_local() {
    let compiled = harness::module(
        "pub fun main(): Int =\n    \
         local fun double(x: Int): Int = x * 2 in\n    \
         let apply = fn(y: Int) -> double(y) in\n    \
         apply(21)\n",
    );

    compiled.validate();

    let (mut store, instance) = instantiate(&compiled);

    assert_eq!(call(&mut store, &instance, "main", &[]), 42);
}

#[test]
fn a_lambda_inside_a_lambda_captures_both_frames() {
    let compiled = harness::module(
        "pub fun main(): Int =\n    \
         let base = 40 in\n    \
         let add = fn(x: Int) -> fn(y: Int) -> x + y + base in\n    \
         add(1)(1)\n",
    );

    compiled.validate();

    let (mut store, instance) = instantiate(&compiled);

    assert_eq!(call(&mut store, &instance, "main", &[]), 42);
}

#[test]
fn a_closure_that_two_branches_produce_joins() {
    let compiled = harness::module(
        "pub fun main(): Int =\n    \
         let base = 40 in\n    \
         let add = if true then fn(x: Int) -> x + base else fn(x: Int) -> x + base in\n    \
         add(2)\n",
    );

    compiled.validate();

    let (mut store, instance) = instantiate(&compiled);

    assert_eq!(call(&mut store, &instance, "main", &[]), 42);
}
