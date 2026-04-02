use super::*;

macro_rules! wrap_simple_native_for_tests {
    ($($name:ident),* $(,)?) => {
        $(
            fn $name(
                args: &[Slot],
                heap: &mut duke_gc::Heap,
                out: &mut dyn std::io::Write,
            ) -> VmResult<Option<Slot>> {
                super::$name(args, heap, out, &mut NativeControl::default())
            }
        )*
    };
}

wrap_simple_native_for_tests!(
    native_arraylist_get,
    native_arrays_copyof_int,
    native_arrays_copyof_object,
    native_arrays_fill_object,
    native_boolean_parseboolean,
    native_double_doublevalue,
    native_double_parsedouble,
    native_float_parsefloat,
    native_hashmap_contains_key,
    native_hashmap_get,
    native_hashmap_get_or_default,
    native_hashmap_init,
    native_hashmap_put,
    native_hashmap_remove,
    native_hashmap_size,
    native_integer_parseint,
    native_long_parselong,
    native_math_min_double,
    native_object_tostring,
    native_print_boolean,
    native_print_char,
    native_print_double,
    native_print_float,
    native_print_long,
    native_print_object,
    native_print_string,
    native_println_boolean,
    native_println_object,
    native_println_string,
    native_sb_append_char,
    native_sb_append_string,
    native_sb_init_string,
    native_string_concat,
    native_string_contains,
    native_string_equals,
    native_string_format,
    native_string_indexof,
    native_string_replace_charsequence,
    native_string_split,
    native_string_substring,
    native_string_substring_range,
    native_string_value_of_object,
    native_system_exit,
);

fn array_list_sort(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn std::io::Write,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    super::array_list_sort(args, heap, out, &mut NativeControl::default(), ops)
}

struct NoopCallbackOps;

impl CallbackOps for NoopCallbackOps {
    fn invoke(
        &mut self,
        _heap: &mut duke_gc::Heap,
        _output: &mut dyn Write,
        _class: &str,
        _method: &str,
        _descriptor: &str,
        _args: Vec<Slot>,
    ) -> VmResult<Option<Slot>> {
        Ok(None)
    }

    fn ensure_loaded(&mut self, _class: &str) -> VmResult<()> {
        Ok(())
    }

    fn inspect_class(&mut self, _class: &str) -> VmResult<ReflectedClassInfo> {
        Ok(ReflectedClassInfo {
            internal_name: String::new(),
            binary_name: String::new(),
            super_class: None,
            interfaces: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
        })
    }
}

struct FixedCodeSourceOps {
    code_source: Option<String>,
}

impl CallbackOps for FixedCodeSourceOps {
    fn invoke(
        &mut self,
        _heap: &mut duke_gc::Heap,
        _output: &mut dyn Write,
        _class: &str,
        _method: &str,
        _descriptor: &str,
        _args: Vec<Slot>,
    ) -> VmResult<Option<Slot>> {
        Ok(None)
    }

    fn ensure_loaded(&mut self, _class: &str) -> VmResult<()> {
        Ok(())
    }

    fn inspect_class(&mut self, _class: &str) -> VmResult<ReflectedClassInfo> {
        Ok(ReflectedClassInfo {
            internal_name: String::new(),
            binary_name: String::new(),
            super_class: None,
            interfaces: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
        })
    }

    fn code_source_for_class(&mut self, _class: &str) -> VmResult<Option<String>> {
        Ok(self.code_source.clone())
    }
}

#[test]
fn native_hashset_init_from_collection_copies_to_array_elements() {
    struct ToArrayOps {
        collection_class: String,
        array_ref: u64,
    }

    impl CallbackOps for ToArrayOps {
        fn invoke(
            &mut self,
            _heap: &mut duke_gc::Heap,
            _output: &mut dyn Write,
            class: &str,
            method: &str,
            descriptor: &str,
            _args: Vec<Slot>,
        ) -> VmResult<Option<Slot>> {
            assert_eq!(class, self.collection_class);
            assert_eq!(method, "toArray");
            assert_eq!(descriptor, "()[Ljava/lang/Object;");
            Ok(Some(Slot::Reference(Some(self.array_ref))))
        }

        fn ensure_loaded(&mut self, _class: &str) -> VmResult<()> {
            Ok(())
        }

        fn inspect_class(&mut self, _class: &str) -> VmResult<ReflectedClassInfo> {
            Ok(ReflectedClassInfo {
                internal_name: String::new(),
                binary_name: String::new(),
                super_class: None,
                interfaces: Vec::new(),
                methods: Vec::new(),
                fields: Vec::new(),
            })
        }
    }

    let mut heap = duke_gc::Heap::new();
    let hashset_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    let collection_ref = heap.allocate("java/util/ImmutableCollections$SetN".to_string(), 0);
    let first = heap.allocate_string("first".to_string());
    let second = heap.allocate_string("second".to_string());
    let array_ref = heap.allocate("[Ljava/lang/Object;".to_string(), 2);
    heap.get_mut(array_ref).unwrap().fields =
        vec![Slot::Reference(Some(first)), Slot::Reference(Some(second))];
    let mut out: Vec<u8> = Vec::new();
    let mut control = NativeControl::default();
    let mut ops = ToArrayOps {
        collection_class: "java/util/ImmutableCollections$SetN".to_string(),
        array_ref,
    };

    native_hashset_init_from_collection(
        &[
            Slot::Reference(Some(hashset_ref)),
            Slot::Reference(Some(collection_ref)),
        ],
        &mut heap,
        &mut out,
        &mut control,
        &mut ops,
    )
    .unwrap();

    let hashset = heap.get(hashset_ref).unwrap();
    assert_eq!(hashset.fields[0], Slot::Int(2));
    assert_eq!(hashset.fields[1], Slot::Reference(Some(first)));
    assert_eq!(hashset.fields[2], Slot::Reference(Some(second)));
}

// ---- Unit tests: hand-crafted instruction streams ----

#[test]
fn class_registry_all_classes_iterates_registered() {
    let reg = ClassRegistry::new();
    // registry starts empty; verify iteration works
    let count = reg.all_classes().count();
    assert_eq!(count, 0); // before bootstrap
}

#[test]
fn native_registry_register_callback_can_be_looked_up() {
    #[allow(clippy::unnecessary_wraps)]
    fn dummy_cb(
        _args: &[Slot],
        _heap: &mut duke_gc::Heap,
        _out: &mut dyn std::io::Write,
        _control: &mut NativeControl,
        _ops: &mut dyn CallbackOps,
    ) -> VmResult<Option<Slot>> {
        Ok(None)
    }
    let mut reg = NativeRegistry::new();
    reg.register_callback("Test", "method", "()V", dummy_cb);
    assert!(matches!(
        reg.get_kind("Test", "method", "()V"),
        Some(HandlerKind::Callback(_))
    ));
}

#[test]
fn native_registry_register_simple_stays_simple() {
    #[allow(clippy::unnecessary_wraps)]
    fn dummy(
        _args: &[Slot],
        _heap: &mut duke_gc::Heap,
        _out: &mut dyn std::io::Write,
        _control: &mut NativeControl,
    ) -> VmResult<Option<Slot>> {
        Ok(None)
    }
    let mut reg = NativeRegistry::new();
    reg.register("Test", "method", "()V", dummy);
    assert!(matches!(
        reg.get_kind("Test", "method", "()V"),
        Some(HandlerKind::Simple(_))
    ));
}

#[test]
fn execute_iconst_ireturn() {
    let instructions = vec![(0, Instruction::Iconst1), (1, Instruction::Ireturn)];
    let result = execute(&instructions, &[], vec![], 2, 1).expect("should execute");
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn execute_bipush_istore_iload() {
    // bipush 42, istore_0, iload_0, ireturn
    let instructions = vec![
        (0, Instruction::Bipush(42)),
        (2, Instruction::Istore0),
        (3, Instruction::Iload0),
        (4, Instruction::Ireturn),
    ];
    let result = execute(&instructions, &[], vec![], 2, 2).unwrap();
    assert_eq!(result, Some(Slot::Int(42)));
}

#[test]
fn execute_loads_args() {
    // iload_0, iload_1, pop, ireturn — returns first arg
    let instructions = vec![
        (0, Instruction::Iload0),
        (1, Instruction::Iload1),
        (2, Instruction::Pop),
        (3, Instruction::Ireturn),
    ];
    let result = execute(&instructions, &[], vec![Slot::Int(99), Slot::Int(0)], 2, 2).unwrap();
    assert_eq!(result, Some(Slot::Int(99)));
}

#[test]
fn execute_sipush() {
    let instructions = vec![(0, Instruction::Sipush(1000)), (3, Instruction::Ireturn)];
    let result = execute(&instructions, &[], vec![], 2, 1).unwrap();
    assert_eq!(result, Some(Slot::Int(1000)));
}

#[test]
fn hand_coded_add() {
    // iload_0, iload_1, iadd, ireturn
    let instrs = vec![
        (0, Instruction::Iload0),
        (1, Instruction::Iload1),
        (2, Instruction::Iadd),
        (3, Instruction::Ireturn),
    ];
    let result = execute(&instrs, &[], vec![Slot::Int(3), Slot::Int(4)], 2, 2).unwrap();
    assert_eq!(result, Some(Slot::Int(7)));
}

#[test]
fn hand_coded_div_by_zero() {
    let instrs = vec![
        (0, Instruction::Iload0),
        (1, Instruction::Iload1),
        (2, Instruction::Idiv),
        (3, Instruction::Ireturn),
    ];
    let err = execute(&instrs, &[], vec![Slot::Int(10), Slot::Int(0)], 2, 2).unwrap_err();
    assert!(matches!(err, VmError::DivisionByZero));
}

#[test]
fn hand_coded_long_add() {
    let instrs = vec![
        (0, Instruction::Lload0),
        (1, Instruction::Lload1),
        (2, Instruction::Ladd),
        (3, Instruction::Lreturn),
    ];
    let result = execute(
        &instrs,
        &[],
        vec![Slot::Long(1_000_000_000), Slot::Long(2_000_000_000)],
        2,
        2,
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Long(3_000_000_000)));
}

#[test]
fn hand_coded_ifeq_taken() {
    // iconst_0 (pc=0), ifeq +5 (pc=1, target=6), iconst_1 (pc=4), ireturn (pc=5),
    // iconst_2 (pc=6), ireturn (pc=7)
    let instrs = vec![
        (0, Instruction::Iconst0),
        (1, Instruction::Ifeq(5)), // 0 == 0, jump to pc=6
        (4, Instruction::Iconst1),
        (5, Instruction::Ireturn),
        (6, Instruction::Iconst2),
        (7, Instruction::Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 2, 1).unwrap();
    assert_eq!(result, Some(Slot::Int(2)));
}

#[test]
fn hand_coded_ifeq_not_taken() {
    let instrs = vec![
        (0, Instruction::Iconst1), // push 1
        (1, Instruction::Ifeq(5)), // 1 != 0, NOT taken
        (4, Instruction::Iconst1), // reached
        (5, Instruction::Ireturn),
        (6, Instruction::Iconst2),
        (7, Instruction::Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 2, 1).unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn hand_coded_goto() {
    // iconst_5 (pc=0), goto +4 (pc=1, target=5), pop (pc=4, skipped),
    // ireturn (pc=5) — returns 5
    let instrs = vec![
        (0, Instruction::Iconst5),
        (1, Instruction::Goto(4)), // jump to pc=5
        (4, Instruction::Pop),     // skipped
        (5, Instruction::Ireturn), // returns 5
    ];
    let result = execute(&instrs, &[], vec![], 2, 1).unwrap();
    assert_eq!(result, Some(Slot::Int(5)));
}

// ---- Integration tests: load Arithmetic.class and execute real bytecode ----

fn fixture(name: &str) -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../tests/fixtures");
    p.push(name);
    p
}

/// Parse and execute a static method from a `.class` file that takes `i32` args
/// and returns an `i32`.
fn run_static_int(class_name: &str, method_name: &str, args: Vec<i32>) -> i32 {
    use duke_bytecode::decode;
    use duke_classfile::{
        parse,
        types::{AttributeData, CpEntry},
    };

    let bytes = std::fs::read(fixture(class_name)).expect("fixture not found");
    let cf = parse(&bytes).expect("parse failed");

    let method = cf
        .methods
        .iter()
        .find(|m| {
            let Some(Some(CpEntry::Utf8(s))) = cf.constant_pool.get(m.name_index.0 as usize)
            else {
                return false;
            };
            s.as_str() == method_name
        })
        .unwrap_or_else(|| panic!("method '{method_name}' not found"));

    let code = method
        .attributes
        .iter()
        .find_map(|a| {
            if let AttributeData::Code(c) = &a.data {
                Some(c)
            } else {
                None
            }
        })
        .expect("no Code attribute");

    let instructions = decode(&code.code).expect("decode failed");
    let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();

    match execute(
        &instructions,
        &cf.constant_pool,
        slots,
        code.max_stack,
        code.max_locals,
    )
    .expect("execute failed")
    {
        Some(Slot::Int(v)) => v,
        other => panic!("unexpected result: {other:?}"),
    }
}

// Basic arithmetic
#[test]
fn int_add() {
    assert_eq!(run_static_int("Arithmetic.class", "add", vec![3, 4]), 7);
}
#[test]
fn int_subtract() {
    assert_eq!(
        run_static_int("Arithmetic.class", "subtract", vec![10, 3]),
        7
    );
}
#[test]
fn int_multiply() {
    assert_eq!(
        run_static_int("Arithmetic.class", "multiply", vec![3, 4]),
        12
    );
}
#[test]
fn int_divide() {
    assert_eq!(run_static_int("Arithmetic.class", "divide", vec![10, 2]), 5);
}
#[test]
fn int_remainder() {
    assert_eq!(
        run_static_int("Arithmetic.class", "remainder", vec![10, 3]),
        1
    );
}
#[test]
fn int_negate() {
    assert_eq!(run_static_int("Arithmetic.class", "negate", vec![-5]), 5);
}
#[test]
fn int_shift_left() {
    assert_eq!(
        run_static_int("Arithmetic.class", "shiftLeft", vec![1, 4]),
        16
    );
}
#[test]
fn int_bitwise_and() {
    assert_eq!(
        run_static_int("Arithmetic.class", "bitwiseAnd", vec![0b1111, 0b1010]),
        0b1010
    );
}
#[test]
fn int_bitwise_or() {
    assert_eq!(
        run_static_int("Arithmetic.class", "bitwiseOr", vec![0b1111, 0b1010]),
        0b1111
    );
}

// Conditionals
#[test]
fn int_max_a_wins() {
    assert_eq!(run_static_int("Arithmetic.class", "max", vec![7, 3]), 7);
}
#[test]
fn int_max_b_wins() {
    assert_eq!(run_static_int("Arithmetic.class", "max", vec![3, 7]), 7);
}
#[test]
fn int_abs_neg() {
    assert_eq!(run_static_int("Arithmetic.class", "abs", vec![-5]), 5);
}
#[test]
fn int_abs_pos() {
    assert_eq!(run_static_int("Arithmetic.class", "abs", vec![5]), 5);
}
#[test]
fn int_clamp_mid() {
    assert_eq!(
        run_static_int("Arithmetic.class", "clamp", vec![5, 1, 10]),
        5
    );
}
#[test]
fn int_clamp_lo() {
    assert_eq!(
        run_static_int("Arithmetic.class", "clamp", vec![0, 1, 10]),
        1
    );
}
#[test]
fn int_clamp_hi() {
    assert_eq!(
        run_static_int("Arithmetic.class", "clamp", vec![15, 1, 10]),
        10
    );
}

// Control flow / loops
#[test]
fn int_factorial_0() {
    assert_eq!(run_static_int("Arithmetic.class", "factorial", vec![0]), 1);
}
#[test]
fn int_factorial_5() {
    assert_eq!(
        run_static_int("Arithmetic.class", "factorial", vec![5]),
        120
    );
}
#[test]
fn int_factorial_10() {
    assert_eq!(
        run_static_int("Arithmetic.class", "factorial", vec![10]),
        3_628_800
    );
}
#[test]
fn int_fibonacci_0() {
    assert_eq!(run_static_int("Arithmetic.class", "fibonacci", vec![0]), 0);
}
#[test]
fn int_fibonacci_1() {
    assert_eq!(run_static_int("Arithmetic.class", "fibonacci", vec![1]), 1);
}
#[test]
fn int_fibonacci_10() {
    assert_eq!(
        run_static_int("Arithmetic.class", "fibonacci", vec![10]),
        55
    );
}
#[test]
fn int_sum_to_100() {
    assert_eq!(run_static_int("Arithmetic.class", "sumTo", vec![100]), 5050);
}

// ---- Phase 5: ClassContext + execute_class() tests ----

fn load_class_context(class_name: &str) -> ClassContext {
    use duke_classfile::parse;
    let bytes = std::fs::read(fixture(class_name)).expect("fixture not found");
    let cf = parse(&bytes).expect("parse failed");
    build_class_context(&cf)
}

fn run_class_int(class_name: &str, method_name: &str, descriptor: &str, args: Vec<i32>) -> i32 {
    let ctx = load_class_context(class_name);
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = duke_loader::DirectoryLoader::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests")
            .join("fixtures"),
    );
    let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    match execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        &entry_class,
        method_name,
        descriptor,
        &slots,
    )
    .expect("execute_class failed")
    {
        Some(Slot::Int(v)) => v,
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn class_square() {
    assert_eq!(
        run_class_int("MathUtils.class", "square", "(I)I", vec![7]),
        49
    );
}

#[test]
fn class_sum_of_squares_3_4() {
    assert_eq!(
        run_class_int("MathUtils.class", "sumOfSquares", "(II)I", vec![3, 4]),
        25
    );
}

#[test]
fn class_sum_of_squares_5_12() {
    assert_eq!(
        run_class_int("MathUtils.class", "sumOfSquares", "(II)I", vec![5, 12]),
        169
    );
}

#[test]
fn class_power_2_10() {
    assert_eq!(
        run_class_int("MathUtils.class", "power", "(II)I", vec![2, 10]),
        1024
    );
}

#[test]
fn class_gcd_48_18() {
    assert_eq!(
        run_class_int("MathUtils.class", "gcd", "(II)I", vec![48, 18]),
        6
    );
}

#[test]
fn class_method_not_found() {
    let ctx = load_class_context("MathUtils.class");
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = duke_loader::DirectoryLoader::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests")
            .join("fixtures"),
    );
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let err = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        &entry_class,
        "nonExistent",
        "(I)I",
        &[],
    )
    .unwrap_err();
    assert!(matches!(err, VmError::MethodNotFound { .. }));
}

#[test]
fn invokevirtual_missing_loaded_method_returns_method_not_found() {
    use duke_classfile::types::CpIndex;
    use std::collections::HashMap;
    use std::sync::Arc;

    let cp = make_cp(vec![
        Some(CpEntry::Methodref {
            class_index: CpIndex(2),
            name_and_type_index: CpIndex(3),
        }),
        Some(CpEntry::Class {
            name_index: CpIndex(4),
        }),
        Some(CpEntry::NameAndType {
            name_index: CpIndex(5),
            descriptor_index: CpIndex(6),
        }),
        Some(CpEntry::Utf8("java/lang/Class".to_string())),
        Some(CpEntry::Utf8("missingMethod".to_string())),
        Some(CpEntry::Utf8("()Ljava/lang/Object;".to_string())),
    ]);
    let instructions: Arc<[(usize, Instruction)]> = vec![
        (0, Instruction::Aload0),
        (1, Instruction::Invokevirtual(CpIndex(1))),
        (4, Instruction::Areturn),
    ]
    .into();
    let method = MethodEntry {
        name: "callMissing".to_string(),
        descriptor: "(Ljava/lang/Class;)Ljava/lang/Object;".to_string(),
        is_public: true,
        is_static: true,
        instructions: Arc::clone(&instructions),
        max_stack: 1,
        max_locals: 1,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (1, 1), (4, 2)])),
    };
    let caller_ctx = ClassContext {
        class_name: "TestInvokevirtualMissing".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: cp,
        methods: vec![method],
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
    };

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.register(caller_ctx);
    let loader = fixtures_loader();
    let class_ref = allocate_class_object(&mut heap, "java/lang/Class").unwrap();
    let mut sink: Vec<u8> = Vec::new();
    let err = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "TestInvokevirtualMissing",
        "callMissing",
        "(Ljava/lang/Class;)Ljava/lang/Object;",
        &[Slot::Reference(Some(class_ref))],
    )
    .unwrap_err();

    assert!(
        matches!(
            err,
            VmError::MethodNotFound { ref name, ref descriptor }
            if name == "java/lang/Class.missingMethod"
                && descriptor == "()Ljava/lang/Object;"
        ),
        "expected MethodNotFound for missing invokevirtual target, got {err:?}"
    );
}

#[test]
fn object_constructor_dispatches_via_invokespecial() {
    use duke_classfile::types::CpIndex;
    use std::collections::HashMap;
    use std::sync::Arc;

    let target_cp = make_cp(vec![
        Some(CpEntry::Methodref {
            class_index: CpIndex(2),
            name_and_type_index: CpIndex(3),
        }),
        Some(CpEntry::Class {
            name_index: CpIndex(4),
        }),
        Some(CpEntry::NameAndType {
            name_index: CpIndex(5),
            descriptor_index: CpIndex(6),
        }),
        Some(CpEntry::Utf8("java/lang/Object".to_string())),
        Some(CpEntry::Utf8("<init>".to_string())),
        Some(CpEntry::Utf8("()V".to_string())),
    ]);
    let target_ctor_instructions: Arc<[(usize, Instruction)]> = vec![
        (0, Instruction::Aload0),
        (1, Instruction::Invokespecial(CpIndex(1))),
        (4, Instruction::Return),
    ]
    .into();
    let target_ctor = MethodEntry {
        name: "<init>".to_string(),
        descriptor: "()V".to_string(),
        is_public: true,
        is_static: false,
        instructions: Arc::clone(&target_ctor_instructions),
        max_stack: 1,
        max_locals: 1,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (1, 1), (4, 2)])),
    };
    let target_ctx = ClassContext {
        class_name: "CtorTarget".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: target_cp,
        methods: vec![target_ctor],
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
    };

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.register(target_ctx);
    let loader = make_simple_loader();
    let mut sink: Vec<u8> = Vec::new();
    let target_ref = heap.allocate("CtorTarget".to_string(), 0);

    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "CtorTarget",
        "<init>",
        "()V",
        &[Slot::Reference(Some(target_ref))],
    );

    assert!(
        result.is_ok(),
        "expected Object.<init>() dispatch to succeed, got {result:?}"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn invokevirtual_dispatches_to_runtime_subclass_implementation() {
    use duke_classfile::types::CpIndex;
    use std::collections::HashMap;
    use std::sync::Arc;

    let caller_cp = make_cp(vec![
        Some(CpEntry::Methodref {
            class_index: CpIndex(2),
            name_and_type_index: CpIndex(3),
        }),
        Some(CpEntry::Class {
            name_index: CpIndex(4),
        }),
        Some(CpEntry::NameAndType {
            name_index: CpIndex(5),
            descriptor_index: CpIndex(6),
        }),
        Some(CpEntry::Utf8("AbstractBase".to_string())),
        Some(CpEntry::Utf8("value".to_string())),
        Some(CpEntry::Utf8("()I".to_string())),
    ]);
    let caller_instructions: Arc<[(usize, Instruction)]> = vec![
        (0, Instruction::Aload0),
        (1, Instruction::Invokevirtual(CpIndex(1))),
        (4, Instruction::Ireturn),
    ]
    .into();
    let caller_method = MethodEntry {
        name: "callValue".to_string(),
        descriptor: "(LAbstractBase;)I".to_string(),
        is_public: true,
        is_static: true,
        instructions: Arc::clone(&caller_instructions),
        max_stack: 1,
        max_locals: 1,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (1, 1), (4, 2)])),
    };
    let caller_ctx = ClassContext {
        class_name: "InvokevirtualCaller".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: caller_cp,
        methods: vec![caller_method],
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
    };
    let abstract_base_ctx = ClassContext {
        class_name: "AbstractBase".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
    };
    let child_instructions: Arc<[(usize, Instruction)]> =
        vec![(0, Instruction::Bipush(7)), (2, Instruction::Ireturn)].into();
    let child_method = MethodEntry {
        name: "value".to_string(),
        descriptor: "()I".to_string(),
        is_public: true,
        is_static: false,
        instructions: Arc::clone(&child_instructions),
        max_stack: 1,
        max_locals: 1,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (2, 1)])),
    };
    let child_ctx = ClassContext {
        class_name: "ConcreteChild".to_string(),
        super_class: Some("AbstractBase".to_string()),
        interfaces: Vec::new(),
        constant_pool: Vec::new(),
        methods: vec![child_method],
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
    };

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.register(caller_ctx);
    registry.register(abstract_base_ctx);
    registry.register(child_ctx);
    let loader = make_simple_loader();
    let mut sink: Vec<u8> = Vec::new();
    let child_ref = heap.allocate("ConcreteChild".to_string(), 0);

    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "InvokevirtualCaller",
        "callValue",
        "(LAbstractBase;)I",
        &[Slot::Reference(Some(child_ref))],
    )
    .expect("invokevirtual should dispatch to ConcreteChild.value");

    assert_eq!(result, Some(Slot::Int(7)));
}

#[test]
#[allow(clippy::too_many_lines)]
fn registered_native_overrides_loaded_bytecode_method() {
    use duke_classfile::types::CpIndex;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
    fn native_value(
        _args: &[Slot],
        _heap: &mut duke_gc::Heap,
        _out: &mut dyn Write,
        _control: &mut NativeControl,
    ) -> VmResult<Option<Slot>> {
        Ok(Some(Slot::Int(42)))
    }

    let caller_cp = make_cp(vec![
        Some(CpEntry::Methodref {
            class_index: CpIndex(2),
            name_and_type_index: CpIndex(3),
        }),
        Some(CpEntry::Class {
            name_index: CpIndex(4),
        }),
        Some(CpEntry::NameAndType {
            name_index: CpIndex(5),
            descriptor_index: CpIndex(6),
        }),
        Some(CpEntry::Utf8("NativeOverrideTarget".to_string())),
        Some(CpEntry::Utf8("value".to_string())),
        Some(CpEntry::Utf8("()I".to_string())),
    ]);
    let caller_instructions: Arc<[(usize, Instruction)]> = vec![
        (0, Instruction::Aload0),
        (1, Instruction::Invokevirtual(CpIndex(1))),
        (4, Instruction::Ireturn),
    ]
    .into();
    let caller_method = MethodEntry {
        name: "callValue".to_string(),
        descriptor: "(LNativeOverrideTarget;)I".to_string(),
        is_public: true,
        is_static: true,
        instructions: Arc::clone(&caller_instructions),
        max_stack: 1,
        max_locals: 1,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (1, 1), (4, 2)])),
    };
    let caller_ctx = ClassContext {
        class_name: "NativeOverrideCaller".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: caller_cp,
        methods: vec![caller_method],
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
    };
    let target_instructions: Arc<[(usize, Instruction)]> =
        vec![(0, Instruction::Bipush(7)), (2, Instruction::Ireturn)].into();
    let target_method = MethodEntry {
        name: "value".to_string(),
        descriptor: "()I".to_string(),
        is_public: true,
        is_static: false,
        instructions: Arc::clone(&target_instructions),
        max_stack: 1,
        max_locals: 1,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (2, 1)])),
    };
    let target_ctx = ClassContext {
        class_name: "NativeOverrideTarget".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: Vec::new(),
        methods: vec![target_method],
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
    };

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry
        .natives_mut()
        .register("NativeOverrideTarget", "value", "()I", native_value);
    registry.register(caller_ctx);
    registry.register(target_ctx);
    let loader = make_simple_loader();
    let mut sink: Vec<u8> = Vec::new();
    let target_ref = heap.allocate("NativeOverrideTarget".to_string(), 0);

    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "NativeOverrideCaller",
        "callValue",
        "(LNativeOverrideTarget;)I",
        &[Slot::Reference(Some(target_ref))],
    )
    .expect("native override should win over loaded bytecode method");

    assert_eq!(result, Some(Slot::Int(42)));
}

#[test]
#[allow(clippy::too_many_lines)]
fn registered_callback_native_overrides_loaded_bytecode_static_method() {
    use duke_classfile::types::CpIndex;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[allow(clippy::unnecessary_wraps)] // must match CallbackNativeHandler signature
    fn native_value(
        _args: &[Slot],
        _heap: &mut duke_gc::Heap,
        _out: &mut dyn Write,
        _control: &mut NativeControl,
        _ops: &mut dyn CallbackOps,
    ) -> VmResult<Option<Slot>> {
        Ok(Some(Slot::Int(42)))
    }

    let caller_cp = make_cp(vec![
        Some(CpEntry::Methodref {
            class_index: CpIndex(2),
            name_and_type_index: CpIndex(3),
        }),
        Some(CpEntry::Class {
            name_index: CpIndex(4),
        }),
        Some(CpEntry::NameAndType {
            name_index: CpIndex(5),
            descriptor_index: CpIndex(6),
        }),
        Some(CpEntry::Utf8("NativeOverrideStaticTarget".to_string())),
        Some(CpEntry::Utf8("value".to_string())),
        Some(CpEntry::Utf8("()I".to_string())),
    ]);
    let caller_instructions: Arc<[(usize, Instruction)]> = vec![
        (0, Instruction::Invokestatic(CpIndex(1))),
        (3, Instruction::Ireturn),
    ]
    .into();
    let caller_method = MethodEntry {
        name: "callValue".to_string(),
        descriptor: "()I".to_string(),
        is_public: true,
        is_static: true,
        instructions: Arc::clone(&caller_instructions),
        max_stack: 1,
        max_locals: 0,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (3, 1)])),
    };
    let caller_ctx = ClassContext {
        class_name: "NativeOverrideStaticCaller".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: caller_cp,
        methods: vec![caller_method],
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
    };
    let target_instructions: Arc<[(usize, Instruction)]> =
        vec![(0, Instruction::Bipush(7)), (2, Instruction::Ireturn)].into();
    let target_method = MethodEntry {
        name: "value".to_string(),
        descriptor: "()I".to_string(),
        is_public: true,
        is_static: true,
        instructions: Arc::clone(&target_instructions),
        max_stack: 1,
        max_locals: 0,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (2, 1)])),
    };
    let target_ctx = ClassContext {
        class_name: "NativeOverrideStaticTarget".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: Vec::new(),
        methods: vec![target_method],
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
    };

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.natives_mut().register_callback(
        "NativeOverrideStaticTarget",
        "value",
        "()I",
        native_value,
    );
    registry.register(caller_ctx);
    registry.register(target_ctx);
    let loader = make_simple_loader();
    let mut sink: Vec<u8> = Vec::new();

    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "NativeOverrideStaticCaller",
        "callValue",
        "()I",
        &[],
    )
    .expect("callback native override should win over loaded bytecode method");

    assert_eq!(result, Some(Slot::Int(42)));
}

// ---- Unit tests: parse_arg_count ----

#[test]
fn arg_count_empty() {
    assert_eq!(parse_arg_count("()V"), 0);
}

#[test]
fn arg_count_single_int() {
    assert_eq!(parse_arg_count("(I)I"), 1);
}

#[test]
fn arg_count_two_ints() {
    assert_eq!(parse_arg_count("(II)I"), 2);
}

#[test]
fn arg_count_long_double() {
    assert_eq!(parse_arg_count("(JD)V"), 2);
}

#[test]
fn arg_count_object_ref() {
    assert_eq!(parse_arg_count("(Ljava/lang/String;I)V"), 2);
}

#[test]
fn arg_count_array() {
    assert_eq!(parse_arg_count("([II)I"), 2);
}

#[test]
fn arg_count_mixed() {
    assert_eq!(parse_arg_count("(ILjava/lang/Object;Z)V"), 3);
}

// ---- Unit tests: resolve_methodref ----

fn make_cp(entries: Vec<Option<CpEntry>>) -> Vec<Option<CpEntry>> {
    let mut cp = vec![None]; // slot 0 reserved
    cp.extend(entries);
    cp
}

#[test]
fn resolve_methodref_valid() {
    use duke_classfile::types::CpIndex;
    let cp = make_cp(vec![
        Some(CpEntry::Methodref {
            class_index: CpIndex(2),
            name_and_type_index: CpIndex(3),
        }),
        Some(CpEntry::Class {
            name_index: CpIndex(6),
        }),
        Some(CpEntry::NameAndType {
            name_index: CpIndex(4),
            descriptor_index: CpIndex(5),
        }),
        Some(CpEntry::Utf8("square".to_string())),
        Some(CpEntry::Utf8("(I)I".to_string())),
        Some(CpEntry::Utf8("MathUtils".to_string())),
    ]);
    let (class_name, name, desc) = resolve_methodref(&cp, 1).unwrap();
    assert_eq!(class_name, "MathUtils");
    assert_eq!(name, "square");
    assert_eq!(desc, "(I)I");
}

#[test]
fn resolve_methodref_invalid_index() {
    let cp = make_cp(vec![]);
    let err = resolve_methodref(&cp, 99).unwrap_err();
    assert!(matches!(err, VmError::InvalidMethodref { index: 99 }));
}

#[test]
fn resolve_methodref_not_a_methodref() {
    let cp = make_cp(vec![Some(CpEntry::Utf8("not a methodref".to_string()))]);
    let err = resolve_methodref(&cp, 1).unwrap_err();
    assert!(matches!(err, VmError::InvalidMethodref { .. }));
}

#[test]
fn build_class_context_initializes_static_defaults_by_descriptor() {
    use duke_classfile::access_flags::{ClassAccessFlags, FieldAccessFlags};
    use duke_classfile::types::{ClassFile, CpIndex, FieldInfo};

    let cf = ClassFile {
        minor_version: 0,
        major_version: 61,
        constant_pool: make_cp(vec![
            Some(CpEntry::Class {
                name_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("StaticDefaults".to_string())),
            Some(CpEntry::Utf8("refField".to_string())),
            Some(CpEntry::Utf8("Ljava/lang/Object;".to_string())),
            Some(CpEntry::Utf8("longField".to_string())),
            Some(CpEntry::Utf8("J".to_string())),
            Some(CpEntry::Utf8("doubleField".to_string())),
            Some(CpEntry::Utf8("D".to_string())),
            Some(CpEntry::Utf8("intField".to_string())),
            Some(CpEntry::Utf8("I".to_string())),
            Some(CpEntry::Utf8("instanceField".to_string())),
            Some(CpEntry::Utf8("[Ljava/lang/String;".to_string())),
        ]),
        access_flags: ClassAccessFlags::empty(),
        this_class: CpIndex(1),
        super_class: CpIndex(0),
        interfaces: Vec::new(),
        fields: vec![
            FieldInfo {
                access_flags: FieldAccessFlags::STATIC,
                name_index: CpIndex(3),
                descriptor_index: CpIndex(4),
                attributes: Vec::new(),
            },
            FieldInfo {
                access_flags: FieldAccessFlags::STATIC,
                name_index: CpIndex(5),
                descriptor_index: CpIndex(6),
                attributes: Vec::new(),
            },
            FieldInfo {
                access_flags: FieldAccessFlags::STATIC,
                name_index: CpIndex(7),
                descriptor_index: CpIndex(8),
                attributes: Vec::new(),
            },
            FieldInfo {
                access_flags: FieldAccessFlags::STATIC,
                name_index: CpIndex(9),
                descriptor_index: CpIndex(10),
                attributes: Vec::new(),
            },
            FieldInfo {
                access_flags: FieldAccessFlags::empty(),
                name_index: CpIndex(11),
                descriptor_index: CpIndex(12),
                attributes: Vec::new(),
            },
        ],
        methods: Vec::new(),
        attributes: Vec::new(),
    };

    let ctx = build_class_context(&cf);

    assert_eq!(ctx.class_name, "StaticDefaults");
    assert_eq!(
        ctx.static_fields,
        vec![
            Slot::Reference(None),
            Slot::Long(0),
            Slot::Double(0.0),
            Slot::Int(0),
        ]
    );
    assert_eq!(ctx.instance_field_count, 1);
}

#[test]
#[allow(clippy::too_many_lines)]
fn hashset_iterator_supports_invokeinterface_iteration() {
    use duke_classfile::types::CpIndex;
    use std::collections::HashMap;
    use std::sync::Arc;

    let cp = make_cp(vec![
        Some(CpEntry::InterfaceMethodref {
            class_index: CpIndex(2),
            name_and_type_index: CpIndex(3),
        }),
        Some(CpEntry::Class {
            name_index: CpIndex(4),
        }),
        Some(CpEntry::NameAndType {
            name_index: CpIndex(5),
            descriptor_index: CpIndex(6),
        }),
        Some(CpEntry::Utf8("java/util/Set".to_string())),
        Some(CpEntry::Utf8("iterator".to_string())),
        Some(CpEntry::Utf8("()Ljava/util/Iterator;".to_string())),
        Some(CpEntry::InterfaceMethodref {
            class_index: CpIndex(8),
            name_and_type_index: CpIndex(9),
        }),
        Some(CpEntry::Class {
            name_index: CpIndex(10),
        }),
        Some(CpEntry::NameAndType {
            name_index: CpIndex(11),
            descriptor_index: CpIndex(12),
        }),
        Some(CpEntry::Utf8("java/util/Iterator".to_string())),
        Some(CpEntry::Utf8("hasNext".to_string())),
        Some(CpEntry::Utf8("()Z".to_string())),
        Some(CpEntry::InterfaceMethodref {
            class_index: CpIndex(8),
            name_and_type_index: CpIndex(14),
        }),
        Some(CpEntry::NameAndType {
            name_index: CpIndex(15),
            descriptor_index: CpIndex(16),
        }),
        Some(CpEntry::Utf8("next".to_string())),
        Some(CpEntry::Utf8("()Ljava/lang/Object;".to_string())),
    ]);
    let instructions: Arc<[(usize, Instruction)]> = vec![
        (
            0,
            Instruction::Aload0, // Set
        ),
        (
            1,
            Instruction::Invokeinterface {
                index: CpIndex(1),
                count: 1,
            },
        ),
        (6, Instruction::Astore1), // Iterator
        (7, Instruction::Aload1),
        (
            8,
            Instruction::Invokeinterface {
                index: CpIndex(7),
                count: 1,
            },
        ),
        (13, Instruction::Ifeq(14)),
        (16, Instruction::Aload1),
        (
            17,
            Instruction::Invokeinterface {
                index: CpIndex(13),
                count: 1,
            },
        ),
        (22, Instruction::Ifnull(5)),
        (25, Instruction::Iconst1),
        (26, Instruction::Ireturn),
        (27, Instruction::Iconst0),
        (28, Instruction::Ireturn),
    ]
    .into();
    let method = MethodEntry {
        name: "iterateOnce".to_string(),
        descriptor: "(Ljava/util/Set;)I".to_string(),
        is_public: true,
        is_static: true,
        instructions: Arc::clone(&instructions),
        max_stack: 1,
        max_locals: 2,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([
            (0, 0),
            (1, 1),
            (6, 2),
            (7, 3),
            (8, 4),
            (13, 5),
            (16, 6),
            (17, 7),
            (22, 8),
            (25, 9),
            (26, 10),
            (27, 11),
            (28, 12),
        ])),
    };
    let caller_ctx = ClassContext {
        class_name: "HashSetIterCaller".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: cp,
        methods: vec![method],
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
    };

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.register(caller_ctx);

    let hashset_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    let value_ref = heap.allocate_string("perm".to_string());
    let mut sink: Vec<u8> = Vec::new();
    native_hashset_init(
        &[Slot::Reference(Some(hashset_ref))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .unwrap();
    native_hashset_add(
        &[
            Slot::Reference(Some(hashset_ref)),
            Slot::Reference(Some(value_ref)),
        ],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .unwrap();

    let loader = make_simple_loader();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "HashSetIterCaller",
        "iterateOnce",
        "(Ljava/util/Set;)I",
        &[Slot::Reference(Some(hashset_ref))],
    );

    assert!(
        result.is_ok(),
        "expected HashSet iterator invokeinterface dispatch to succeed, got {result:?}"
    );
    assert_eq!(result.unwrap(), Some(Slot::Int(1)));
}

// ---- Phase 6: object creation + field access ----

#[test]
fn point_sum_1_2_3_4() {
    assert_eq!(
        run_class_int("Point.class", "sumPoints", "(IIII)I", vec![1, 2, 3, 4]),
        10
    );
}

#[test]
fn point_sum_3_4_0_0() {
    assert_eq!(
        run_class_int("Point.class", "sumPoints", "(IIII)I", vec![3, 4, 0, 0]),
        7
    );
}

#[test]
fn point_sum_zeros() {
    assert_eq!(
        run_class_int("Point.class", "sumPoints", "(IIII)I", vec![0, 0, 0, 0]),
        0
    );
}

#[test]
fn point_sum_symmetry() {
    let a = run_class_int("Point.class", "sumPoints", "(IIII)I", vec![1, 2, 3, 4]);
    let b = run_class_int("Point.class", "sumPoints", "(IIII)I", vec![3, 4, 1, 2]);
    assert_eq!(a, b);
}

// ---- Unit tests: resolve_fieldref ----

#[test]
fn resolve_fieldref_valid() {
    use duke_classfile::types::CpIndex;
    let cp = make_cp(vec![
        Some(CpEntry::Fieldref {
            class_index: CpIndex(2),
            name_and_type_index: CpIndex(3),
        }),
        Some(CpEntry::Class {
            name_index: CpIndex(6),
        }),
        Some(CpEntry::NameAndType {
            name_index: CpIndex(4),
            descriptor_index: CpIndex(5),
        }),
        Some(CpEntry::Utf8("x".to_string())),
        Some(CpEntry::Utf8("I".to_string())),
        Some(CpEntry::Utf8("Point".to_string())),
    ]);
    let (class_name, name, desc) = resolve_fieldref(&cp, 1).unwrap();
    assert_eq!(class_name, "Point");
    assert_eq!(name, "x");
    assert_eq!(desc, "I");
}

#[test]
fn resolve_fieldref_invalid() {
    let cp = make_cp(vec![Some(CpEntry::Utf8("not a fieldref".to_string()))]);
    let err = resolve_fieldref(&cp, 1).unwrap_err();
    assert!(matches!(err, VmError::InvalidFieldref { .. }));
}

// ---- Phase 7: Arrays ----

fn run_class_long(
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: Vec<i32>,
) -> i64 {
    let ctx = load_class_context(class_name);
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = duke_loader::DirectoryLoader::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests")
            .join("fixtures"),
    );
    let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    match execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        &entry_class,
        method_name,
        descriptor,
        &slots,
    )
    .expect("execute_class failed")
    {
        Some(Slot::Long(v)) => v,
        other => panic!("unexpected result: {other:?}"),
    }
}

fn run_class_double(
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: Vec<i32>,
) -> f64 {
    let ctx = load_class_context(class_name);
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = duke_loader::DirectoryLoader::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests")
            .join("fixtures"),
    );
    let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    match execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        &entry_class,
        method_name,
        descriptor,
        &slots,
    )
    .expect("execute_class failed")
    {
        Some(Slot::Double(v)) => v,
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn array_sum_5() {
    // sumArray(5) = 1+2+3+4+5 = 15
    assert_eq!(
        run_class_int("ArrayOps.class", "sumArray", "(I)I", vec![5]),
        15
    );
}

#[test]
fn array_length() {
    assert_eq!(
        run_class_int("ArrayOps.class", "arrayLength", "(I)I", vec![7]),
        7
    );
}

#[test]
fn array_sum_long() {
    // sumLongArray(3) = 0*1e6 + 1*1e6 + 2*1e6 = 3_000_000
    assert_eq!(
        run_class_long("ArrayOps.class", "sumLongArray", "(I)J", vec![3]),
        3_000_000
    );
}

#[test]
fn array_first_double() {
    // firstDouble(3): a[0] = 0 * 0.5 = 0.0
    let result = run_class_double("ArrayOps.class", "firstDouble", "(I)D", vec![3]);
    assert!((result - 0.0).abs() < f64::EPSILON);
}

#[test]
fn array_empty_sum() {
    // sumArray(0) = sum of empty = 0
    assert_eq!(
        run_class_int("ArrayOps.class", "sumArray", "(I)I", vec![0]),
        0
    );
}

// ---- Phase 7: Array unit tests (no fixture needed) ----

#[test]
fn newarray_int_arraylength() {
    // newarray T_INT count=3 -> arraylength -> 3
    use duke_bytecode::Instruction::*;
    let instrs = vec![
        (0, Iconst3),
        (1, Newarray(ArrayType::Int)),
        (3, Arraylength),
        (4, Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 3, 0).unwrap();
    assert_eq!(result, Some(Slot::Int(3)));
}

#[test]
fn newarray_iastore_iaload() {
    use duke_bytecode::Instruction::*;
    // int[] a = new int[1]; a[0] = 42; return a[0];
    let instrs = vec![
        (0, Iconst1),
        (1, Newarray(ArrayType::Int)),
        (3, Dup),
        (4, Iconst0),
        (5, Bipush(42)),
        (7, Iastore),
        (8, Iconst0),
        (9, Iaload),
        (10, Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 4, 0).unwrap();
    assert_eq!(result, Some(Slot::Int(42)));
}

#[test]
fn newarray_int_bounds_error() {
    use duke_bytecode::Instruction::*;
    // new int[1], then iaload at index 5 -> ArrayIndexOutOfBounds
    let instrs = vec![
        (0, Iconst1),
        (1, Newarray(ArrayType::Int)),
        (3, Bipush(5)),
        (5, Iaload),
        (6, Ireturn),
    ];
    let err = execute(&instrs, &[], vec![], 3, 0).unwrap_err();
    assert!(matches!(
        err,
        VmError::ArrayIndexOutOfBounds {
            index: 5,
            length: 1
        }
    ));
}

#[test]
fn test_extract_int_arg_missing() {
    let args = vec![];
    assert!(matches!(
        extract_int_arg(&args, 0).unwrap_err(),
        VmError::TypeMismatch {
            expected: "Int",
            got: "other"
        }
    ));
}

#[test]
fn newarray_negative_size() {
    use duke_bytecode::Instruction::*;
    let instrs = vec![
        (0, IconstM1),
        (1, Newarray(ArrayType::Int)),
        (3, Arraylength),
        (4, Ireturn),
    ];
    let err = execute(&instrs, &[], vec![], 2, 0).unwrap_err();
    assert!(matches!(err, VmError::NegativeArraySize { size: -1 }));
}

#[test]
fn athrow_propagates_as_java_exception() {
    use duke_bytecode::Instruction::*;
    let instrs = vec![(0, Iconst1), (1, Newarray(ArrayType::Int)), (3, Athrow)];
    let err = execute(&instrs, &[], vec![], 2, 0).unwrap_err();
    assert!(matches!(err, VmError::JavaException { .. }));
}

// ---- Phase 8: Exceptions ----

#[test]
fn exception_catch_simple() {
    assert_eq!(
        run_class_int("ExceptionTest.class", "catchSimple", "()I", vec![]),
        42
    );
}

#[test]
fn exception_uncaught_propagates() {
    let ctx = load_class_context("ExceptionTest.class");
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = duke_loader::DirectoryLoader::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests")
            .join("fixtures"),
    );
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        &entry_class,
        "uncaught",
        "()I",
        &[],
    );
    let err = result.unwrap_err();
    assert!(matches!(err, VmError::JavaException { .. }));
}

#[test]
fn exception_finally_block() {
    assert_eq!(
        run_class_int("ExceptionTest.class", "finallyBlock", "()I", vec![]),
        11 // 10 + 1
    );
}

#[test]
fn exception_catch_from_callee() {
    assert_eq!(
        run_class_int("ExceptionTest.class", "catchFromCallee", "()I", vec![]),
        99
    );
}

// ---- Phase 8: Switch statements ----

#[test]
fn switch_dense_case0() {
    assert_eq!(
        run_class_int("ExceptionTest.class", "switchDense", "(I)I", vec![0]),
        10
    );
}

#[test]
fn switch_dense_case2() {
    assert_eq!(
        run_class_int("ExceptionTest.class", "switchDense", "(I)I", vec![2]),
        30
    );
}

#[test]
fn switch_dense_default() {
    assert_eq!(
        run_class_int("ExceptionTest.class", "switchDense", "(I)I", vec![99]),
        -1
    );
}

#[test]
fn switch_sparse_case200() {
    assert_eq!(
        run_class_int("ExceptionTest.class", "switchSparse", "(I)I", vec![200]),
        2
    );
}

#[test]
fn switch_sparse_default() {
    assert_eq!(
        run_class_int("ExceptionTest.class", "switchSparse", "(I)I", vec![999]),
        0
    );
}

// ---- Phase 8: Switch (unit tests) ----

#[test]
fn tableswitch_match() {
    use duke_bytecode::Instruction::*;
    let instrs = vec![
        (0, Iconst1),
        (
            1,
            Tableswitch {
                default: 100,
                low: 0,
                high: 2,
                offsets: vec![10, 20, 30],
            },
        ),
        (11, Bipush(10)),
        (13, Ireturn),
        (21, Bipush(20)),
        (23, Ireturn),
        (31, Bipush(30)),
        (33, Ireturn),
        (101, Bipush(-1)),
        (103, Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 2, 0).unwrap();
    assert_eq!(result, Some(Slot::Int(20))); // case 1
}

#[test]
fn tableswitch_default() {
    use duke_bytecode::Instruction::*;
    let instrs = vec![
        (0, Bipush(99)),
        (
            2,
            Tableswitch {
                default: 100,
                low: 0,
                high: 2,
                offsets: vec![10, 20, 30],
            },
        ),
        (12, Bipush(10)),
        (14, Ireturn),
        (22, Bipush(20)),
        (24, Ireturn),
        (32, Bipush(30)),
        (34, Ireturn),
        (102, Bipush(-1)),
        (104, Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 2, 0).unwrap();
    assert_eq!(result, Some(Slot::Int(-1)));
}

#[test]
fn lookupswitch_match() {
    use duke_bytecode::Instruction::*;
    let instrs = vec![
        (0, Sipush(200)),
        (
            3,
            Lookupswitch {
                default: 100,
                pairs: vec![(100, 10), (200, 20), (300, 30)],
            },
        ),
        (13, Bipush(1)),
        (15, Ireturn),
        (23, Bipush(2)),
        (25, Ireturn),
        (33, Bipush(3)),
        (35, Ireturn),
        (103, Bipush(0)),
        (105, Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 2, 0).unwrap();
    assert_eq!(result, Some(Slot::Int(2)));
}

#[test]
fn lookupswitch_default() {
    use duke_bytecode::Instruction::*;
    let instrs = vec![
        (0, Sipush(999)),
        (
            3,
            Lookupswitch {
                default: 100,
                pairs: vec![(100, 10), (200, 20), (300, 30)],
            },
        ),
        (13, Bipush(1)),
        (15, Ireturn),
        (23, Bipush(2)),
        (25, Ireturn),
        (33, Bipush(3)),
        (35, Ireturn),
        (103, Bipush(0)),
        (105, Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 2, 0).unwrap();
    assert_eq!(result, Some(Slot::Int(0)));
}

// ---- Phase 9: String constants ----

#[test]
fn string_non_null() {
    let result = run_class_int("StringAndTypes.class", "stringNonNull", "()I", vec![]);
    assert_eq!(result, 1);
}

#[test]
fn string_intern() {
    let result = run_class_int("StringAndTypes.class", "stringIntern", "()I", vec![]);
    assert_eq!(result, 1);
}

// ---- Phase 9: instanceof ----

#[test]
fn instanceof_match() {
    let result = run_class_int("StringAndTypes.class", "instanceOfMatch", "()I", vec![]);
    assert_eq!(result, 1);
}

#[test]
fn instanceof_mismatch() {
    let result = run_class_int("StringAndTypes.class", "instanceOfMismatch", "()I", vec![]);
    assert_eq!(result, 0);
}

#[test]
fn instanceof_null() {
    let result = run_class_int("StringAndTypes.class", "instanceOfNull", "()I", vec![]);
    assert_eq!(result, 0);
}

// ---- Phase 9: checkcast ----

#[test]
fn checkcast_ok() {
    let result = run_class_int("StringAndTypes.class", "checkcastOk", "()I", vec![]);
    assert_eq!(result, 42);
}

// ---- Phase 9: Reference comparison ----

#[test]
fn ref_equal() {
    let result = run_class_int("StringAndTypes.class", "refEqual", "()I", vec![]);
    assert_eq!(result, 1);
}

#[test]
fn ref_not_equal() {
    let result = run_class_int("StringAndTypes.class", "refNotEqual", "()I", vec![]);
    assert_eq!(result, 1);
}

// ---- Phase 9: if_acmpeq / if_acmpne unit tests ----

#[test]
fn if_acmpeq_same_ref() {
    use duke_bytecode::Instruction::*;
    let instrs = vec![
        (0, Iconst1),
        (1, Newarray(ArrayType::Int)),
        (3, Dup),
        (4, IfAcmpeq(10)),
        (7, Iconst0),
        (8, Ireturn),
        (14, Iconst1),
        (15, Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 3, 0).unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn if_acmpne_different_refs() {
    use duke_bytecode::Instruction::*;
    let instrs = vec![
        (0, Iconst1),
        (1, Newarray(ArrayType::Int)),
        (3, Iconst1),
        (4, Newarray(ArrayType::Int)),
        (6, IfAcmpne(10)),
        (9, Iconst0),
        (10, Ireturn),
        (16, Iconst1),
        (17, Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 3, 0).unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

// ---- Phase 10: Cross-class dispatch ----

fn run_cross_class_int(
    class_files: &[&str],
    entry_class: &str,
    method_name: &str,
    descriptor: &str,
    args: Vec<i32>,
) -> i32 {
    let mut registry = ClassRegistry::new();
    for name in class_files {
        let ctx = load_class_context(name);
        registry.register(ctx);
    }
    let loader = duke_loader::DirectoryLoader::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests")
            .join("fixtures"),
    );
    let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    match execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        entry_class,
        method_name,
        descriptor,
        &slots,
    )
    .expect("execute_class failed")
    {
        Some(Slot::Int(v)) => v,
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn cross_class_add() {
    assert_eq!(
        run_cross_class_int(
            &["CrossCall.class", "Callee.class"],
            "CrossCall",
            "callAdd",
            "(II)I",
            vec![3, 4],
        ),
        7
    );
}

#[test]
fn cross_class_double() {
    assert_eq!(
        run_cross_class_int(
            &["CrossCall.class", "Callee.class"],
            "CrossCall",
            "callDouble",
            "(I)I",
            vec![5],
        ),
        10
    );
}

#[test]
fn cross_class_chain() {
    assert_eq!(
        run_cross_class_int(
            &["CrossCall.class", "Callee.class"],
            "CrossCall",
            "chainCall",
            "(I)I",
            vec![3],
        ),
        9
    );
}

#[test]
fn cross_class_add_negated() {
    assert_eq!(
        run_cross_class_int(
            &["CrossCall.class", "Callee.class"],
            "CrossCall",
            "addNegated",
            "(I)I",
            vec![5],
        ),
        0
    );
}

#[test]
fn cross_class_pair_sum() {
    assert_eq!(
        run_cross_class_int(
            &["PairUser.class", "Pair.class"],
            "PairUser",
            "makePairSum",
            "(II)I",
            vec![3, 7],
        ),
        10
    );
}

#[test]
fn cross_class_pair_diff() {
    assert_eq!(
        run_cross_class_int(
            &["PairUser.class", "Pair.class"],
            "PairUser",
            "makePairDiff",
            "(II)I",
            vec![10, 3],
        ),
        7
    );
}

#[test]
fn cross_class_two_pairs() {
    assert_eq!(
        run_cross_class_int(
            &["PairUser.class", "Pair.class"],
            "PairUser",
            "twoPairsSum",
            "(IIII)I",
            vec![1, 2, 3, 4],
        ),
        10
    );
}

#[test]
fn native_registry_stores_and_retrieves() {
    #[allow(clippy::unnecessary_wraps)]
    fn dummy_handler(
        _args: &[Slot],
        _heap: &mut duke_gc::Heap,
        _out: &mut dyn std::io::Write,
        _control: &mut NativeControl,
    ) -> VmResult<Option<Slot>> {
        Ok(Some(Slot::Int(99)))
    }
    let mut natives = NativeRegistry::new();
    natives.register("Foo", "bar", "(I)I", dummy_handler);
    let handler = natives.get("Foo", "bar", "(I)I");
    assert!(handler.is_some());
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let result = handler.unwrap()(
        &[Slot::Int(1)],
        &mut heap,
        &mut out,
        &mut NativeControl::default(),
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(99)));
}

#[test]
fn native_registry_returns_none_for_missing() {
    let natives = NativeRegistry::new();
    assert!(natives.get("Foo", "bar", "(I)I").is_none());
}

// ---- Phase 11: native println tests ----

fn load_hello_class() -> ClassContext {
    let bytes = std::fs::read(fixture("Hello.class")).expect("Hello.class");
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

#[test]
fn hello_greet_prints_to_output() {
    let ctx = load_hello_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = duke_loader::DirectoryLoader::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests")
            .join("fixtures"),
    );
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "Hello",
        "greet",
        "()V",
        &[],
    )
    .unwrap();
    assert_eq!(result, None);
    assert_eq!(String::from_utf8_lossy(&out), "Hello, Duke!\n");
}

#[test]
fn hello_print_num() {
    let ctx = load_hello_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = duke_loader::DirectoryLoader::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests")
            .join("fixtures"),
    );
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "Hello",
        "printNum",
        "(I)V",
        &[Slot::Int(42)],
    )
    .unwrap();
    assert_eq!(result, None);
    assert_eq!(String::from_utf8_lossy(&out), "42\n");
}

#[test]
fn hello_greet_and_return() {
    let ctx = load_hello_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = duke_loader::DirectoryLoader::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests")
            .join("fixtures"),
    );
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "Hello",
        "greetAndReturn",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(42)));
    assert_eq!(String::from_utf8_lossy(&out), "Greetings!\n");
}

#[test]
fn hello_blank_line() {
    let ctx = load_hello_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = duke_loader::DirectoryLoader::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests")
            .join("fixtures"),
    );
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "Hello",
        "blankLine",
        "()V",
        &[],
    )
    .unwrap();
    assert_eq!(result, None);
    assert_eq!(String::from_utf8_lossy(&out), "\n");
}

// ---- Phase 12: stack manipulation tests ----

#[test]
fn stack_ops_dup_x1() {
    // dup_x1: ..., v2, v1 → ..., v1, v2, v1
    let instrs = vec![
        (0, Instruction::Iconst2), // push 2 (v2)
        (1, Instruction::Iconst3), // push 3 (v1)
        (2, Instruction::DupX1),   // → 3, 2, 3
        (3, Instruction::Iadd),    // → 3, 5
        (4, Instruction::Iadd),    // → 8
        (5, Instruction::Ireturn),
    ];
    let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
    assert_eq!(r, Some(Slot::Int(8)));
}

#[test]
fn stack_ops_dup_x2() {
    // dup_x2: ..., v3, v2, v1 → ..., v1, v3, v2, v1
    let instrs = vec![
        (0, Instruction::Iconst1), // push 1 (v3)
        (1, Instruction::Iconst2), // push 2 (v2)
        (2, Instruction::Iconst3), // push 3 (v1)
        (3, Instruction::DupX2),   // → 3, 1, 2, 3
        (4, Instruction::Iadd),    // → 3, 1, 5
        (5, Instruction::Iadd),    // → 3, 6
        (6, Instruction::Iadd),    // → 9
        (7, Instruction::Ireturn),
    ];
    let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
    assert_eq!(r, Some(Slot::Int(9)));
}

#[test]
fn stack_ops_dup2() {
    // dup2: ..., v2, v1 → ..., v2, v1, v2, v1
    let instrs = vec![
        (0, Instruction::Iconst4), // push 4 (v2)
        (1, Instruction::Iconst5), // push 5 (v1)
        (2, Instruction::Dup2),    // → 4, 5, 4, 5
        (3, Instruction::Iadd),    // → 4, 5, 9
        (4, Instruction::Iadd),    // → 4, 14
        (5, Instruction::Iadd),    // → 18
        (6, Instruction::Ireturn),
    ];
    let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
    assert_eq!(r, Some(Slot::Int(18)));
}

#[test]
fn stack_ops_dup2_x1() {
    // dup2_x1: ..., v3, v2, v1 → ..., v2, v1, v3, v2, v1
    let instrs = vec![
        (0, Instruction::Iconst1), // 1 (v3)
        (1, Instruction::Iconst2), // 2 (v2)
        (2, Instruction::Iconst3), // 3 (v1)
        (3, Instruction::Dup2X1),  // → 2, 3, 1, 2, 3
        (4, Instruction::Iadd),    // → 2, 3, 1, 5
        (5, Instruction::Iadd),    // → 2, 3, 6
        (6, Instruction::Iadd),    // → 2, 9
        (7, Instruction::Iadd),    // → 11
        (8, Instruction::Ireturn),
    ];
    let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
    assert_eq!(r, Some(Slot::Int(11)));
}

#[test]
fn stack_ops_dup2_x2() {
    // dup2_x2: ..., v4, v3, v2, v1 → ..., v2, v1, v4, v3, v2, v1
    let instrs = vec![
        (0, Instruction::Iconst1), // 1 (v4)
        (1, Instruction::Iconst2), // 2 (v3)
        (2, Instruction::Iconst3), // 3 (v2)
        (3, Instruction::Iconst4), // 4 (v1)
        (4, Instruction::Dup2X2),  // → 3, 4, 1, 2, 3, 4
        (5, Instruction::Iadd),    // → 3, 4, 1, 2, 7
        (6, Instruction::Iadd),    // → 3, 4, 1, 9
        (7, Instruction::Iadd),    // → 3, 4, 10
        (8, Instruction::Iadd),    // → 3, 14
        (9, Instruction::Iadd),    // → 17
        (10, Instruction::Ireturn),
    ];
    let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
    assert_eq!(r, Some(Slot::Int(17)));
}

// ---- Phase 12: static initializer tests ----

fn load_static_init_class() -> ClassContext {
    let bytes = std::fs::read(fixture("StaticInit.class")).expect("StaticInit.class");
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

fn fixtures_loader() -> duke_loader::DirectoryLoader {
    duke_loader::DirectoryLoader::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests")
            .join("fixtures"),
    )
}

#[test]
fn clinit_initializes_static_field_x() {
    let ctx = load_static_init_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = fixtures_loader();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StaticInit",
        "getX",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(42)));
}

#[test]
fn clinit_initializes_dependent_field_y() {
    let ctx = load_static_init_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = fixtures_loader();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StaticInit",
        "getY",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(50)));
}

#[test]
fn clinit_runs_static_block() {
    let ctx = load_static_init_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = fixtures_loader();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StaticInit",
        "getZ",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(92)));
}

#[test]
fn clinit_sum_all_statics() {
    let ctx = load_static_init_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = fixtures_loader();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StaticInit",
        "sum",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(184)));
}

// ---- Phase 12: stack ops integration tests ----

fn load_stack_ops_class() -> ClassContext {
    let bytes = std::fs::read(fixture("StackOps.class")).expect("StackOps.class");
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

#[test]
fn stack_ops_array_store_dup() {
    let ctx = load_stack_ops_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = fixtures_loader();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StackOps",
        "arrayStoreDup",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(60)));
}

#[test]
fn stack_ops_multi_assign() {
    let ctx = load_stack_ops_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = fixtures_loader();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StackOps",
        "multiAssign",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(10)));
}

// ---- Phase 13: class hierarchy tests ----

fn load_hierarchy_class() -> ClassContext {
    let bytes = std::fs::read(fixture("Hierarchy.class")).expect("Hierarchy.class");
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

fn load_exception_hierarchy_class() -> ClassContext {
    let bytes =
        std::fs::read(fixture("ExceptionHierarchy.class")).expect("ExceptionHierarchy.class");
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

fn load_string_ops_class() -> ClassContext {
    let bytes = std::fs::read(fixture("StringOps.class")).expect("StringOps.class");
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

#[test]
fn hierarchy_instanceof_object() {
    let ctx = load_hierarchy_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "Hierarchy",
        "instanceOfObject",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn hierarchy_cast_to_object() {
    let ctx = load_hierarchy_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "Hierarchy",
        "castToObject",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn hierarchy_null_instanceof() {
    let ctx = load_hierarchy_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "Hierarchy",
        "nullInstanceOf",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(0)));
}

// ---- Phase 13: exception hierarchy tests ----

#[test]
fn exception_hierarchy_catch_parent() {
    let ctx = load_exception_hierarchy_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "ExceptionHierarchy",
        "catchParent",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn exception_hierarchy_catch_exact() {
    let ctx = load_exception_hierarchy_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "ExceptionHierarchy",
        "catchExact",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(2)));
}

#[test]
fn exception_hierarchy_catch_wrong_then_right() {
    let ctx = load_exception_hierarchy_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "ExceptionHierarchy",
        "catchWrongThenRight",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(3)));
}

#[test]
fn exception_hierarchy_catch_grandparent() {
    let ctx = load_exception_hierarchy_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "ExceptionHierarchy",
        "catchGrandparent",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(4)));
}

// ---- Phase 13: string ops tests ----

#[test]
fn string_ops_length() {
    let ctx = load_string_ops_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StringOps",
        "stringLength",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(5)));
}

#[test]
fn string_ops_equals() {
    let ctx = load_string_ops_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StringOps",
        "stringEquals",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn test_archive_ref_from_slot_success() {
    let mut heap = duke_gc::Heap::new();
    let obj_ref = heap.allocate("duke/net/Socket".to_string(), 1);
    let file_ref = heap.allocate("java/io/File".to_string(), 1);
    heap.get_mut(obj_ref).unwrap().fields[0] = Slot::Reference(Some(file_ref));
    assert_eq!(
        archive_ref_from_slot(&heap, obj_ref, 0).unwrap(),
        Some(file_ref)
    );
}

#[test]
fn test_archive_ref_from_slot_null() {
    let mut heap = duke_gc::Heap::new();
    let obj_ref = heap.allocate("duke/net/Socket".to_string(), 1);
    heap.get_mut(obj_ref).unwrap().fields[0] = Slot::Reference(None);
    assert_eq!(archive_ref_from_slot(&heap, obj_ref, 0).unwrap(), None);
}

#[test]
fn test_archive_ref_from_slot_none() {
    let mut heap = duke_gc::Heap::new();
    let obj_ref = heap.allocate("duke/net/Socket".to_string(), 1);
    // Field 1 doesn't exist
    assert_eq!(archive_ref_from_slot(&heap, obj_ref, 1).unwrap(), None);
}

#[test]
fn test_archive_ref_from_slot_type_mismatch() {
    let mut heap = duke_gc::Heap::new();
    let obj_ref = heap.allocate("duke/net/Socket".to_string(), 1);
    heap.get_mut(obj_ref).unwrap().fields[0] = Slot::Int(42);
    assert!(matches!(
        archive_ref_from_slot(&heap, obj_ref, 0),
        Err(VmError::TypeMismatch {
            expected: "Reference",
            ..
        })
    ));
}

#[test]
fn test_archive_path_from_slot_none() {
    let mut heap = duke_gc::Heap::new();
    let archive_ref = heap.allocate(
        "org/springframework/boot/loader/launch/JarFileArchive".to_string(),
        1,
    );
    heap.get_mut(archive_ref).unwrap().fields[0] = Slot::Reference(None);
    assert_eq!(archive_path_from_slot(&heap, archive_ref, 0).unwrap(), None);
}

#[test]
fn test_boot_archive_path_from_ref_exploded_archive() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();

    let exploded_ctx = ClassContext {
        class_name: "org/springframework/boot/loader/launch/ExplodedArchive".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "rootDirectory".to_string(),
            descriptor: "Ljava/io/File;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(exploded_ctx);

    let file_ref = heap.allocate("java/io/File".to_string(), 1);
    let str_ref = heap.allocate_string("/path/to/exploded".to_string());
    heap.get_mut(file_ref).unwrap().fields[0] = Slot::Reference(Some(str_ref));

    let archive_ref = heap.allocate(
        "org/springframework/boot/loader/launch/ExplodedArchive".to_string(),
        1,
    );
    heap.get_mut(archive_ref).unwrap().fields[0] = Slot::Reference(Some(file_ref));

    let path = boot_archive_path_from_ref(&registry, &heap, archive_ref).unwrap();
    assert_eq!(path, Some("/path/to/exploded".to_string()));
}

#[test]
fn test_boot_archive_path_from_ref_unknown_class() {
    let registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();

    let archive_ref = heap.allocate("duke/net/Socket".to_string(), 1);
    let path = boot_archive_path_from_ref(&registry, &heap, archive_ref).unwrap();
    assert_eq!(path, None);
}

#[test]
fn test_launched_class_loader_archive_path_wrong_class() {
    let registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();

    let loader_ref = heap.allocate("java/net/URLClassLoader".to_string(), 1);
    let path = launched_class_loader_archive_path(&registry, &heap, loader_ref).unwrap();
    assert_eq!(path, None);
}

#[test]
fn string_ops_not_equals() {
    let ctx = load_string_ops_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StringOps",
        "stringNotEquals",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(0)));
}

#[test]
fn string_ops_char_at() {
    let ctx = load_string_ops_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StringOps",
        "charAtOne",
        "()I",
        &[],
    )
    .unwrap();
    // 'e' = 101
    assert_eq!(result, Some(Slot::Int(101)));
}

// ---- Phase 14: interface tests ----

fn load_class(name: &str) -> ClassContext {
    let bytes = std::fs::read(fixture(name)).unwrap_or_else(|_| panic!("{name}"));
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

#[test]
fn interface_call_simple() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("InterfaceTest.class"));
    registry.register(load_class("InterfaceTest$Adder.class"));
    registry.register(load_class("InterfaceTest$SimpleAdder.class"));
    registry.register(load_class("InterfaceTest$DoubleAdder.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "InterfaceTest",
        "callSimple",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(7)));
}

#[test]
fn interface_call_double() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("InterfaceTest.class"));
    registry.register(load_class("InterfaceTest$Adder.class"));
    registry.register(load_class("InterfaceTest$SimpleAdder.class"));
    registry.register(load_class("InterfaceTest$DoubleAdder.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "InterfaceTest",
        "callDouble",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(14)));
}

#[test]
fn interface_polymorphic_simple() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("InterfaceTest.class"));
    registry.register(load_class("InterfaceTest$Adder.class"));
    registry.register(load_class("InterfaceTest$SimpleAdder.class"));
    registry.register(load_class("InterfaceTest$DoubleAdder.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "InterfaceTest",
        "polymorphic",
        "(I)I",
        &[Slot::Int(0)],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(8)));
}

#[test]
fn interface_polymorphic_double() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("InterfaceTest.class"));
    registry.register(load_class("InterfaceTest$Adder.class"));
    registry.register(load_class("InterfaceTest$SimpleAdder.class"));
    registry.register(load_class("InterfaceTest$DoubleAdder.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "InterfaceTest",
        "polymorphic",
        "(I)I",
        &[Slot::Int(1)],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(16)));
}

// ---- Phase 14: multi-dimensional array tests ----

#[test]
fn multiarray_sum2d() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("MultiArray.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "MultiArray",
        "sum2d",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(21)));
}

#[test]
fn multiarray_dimensions() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("MultiArray.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "MultiArray",
        "dimensions",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(12)));
}

// ---- Phase 14: native method tests ----

#[test]
fn more_natives_object_hashcode() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("MoreNatives.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "MoreNatives",
        "objectHashCode",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn more_natives_value_of_int() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("MoreNatives.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "MoreNatives",
        "valueOfInt",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(2)));
}

#[test]
fn more_natives_print_no_newline() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("MoreNatives.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "MoreNatives",
        "printNoNewline",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
    assert_eq!(String::from_utf8_lossy(&out), "ABCD\n");
}

// ---- Phase 15: inherited method tests ----

fn load_inherited_method_classes(registry: &mut ClassRegistry) {
    registry.register(load_class("InheritedMethod.class"));
    registry.register(load_class("InheritedMethod$Animal.class"));
    registry.register(load_class("InheritedMethod$Dog.class"));
    registry.register(load_class("InheritedMethod$Puppy.class"));
}

#[test]
fn inherited_call_inherited() {
    let mut registry = ClassRegistry::new();
    load_inherited_method_classes(&mut registry);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "InheritedMethod",
        "callInherited",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(10)));
}

#[test]
fn inherited_call_overridden() {
    let mut registry = ClassRegistry::new();
    load_inherited_method_classes(&mut registry);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "InheritedMethod",
        "callOverridden",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(2)));
}

#[test]
fn inherited_call_deep_inherited() {
    let mut registry = ClassRegistry::new();
    load_inherited_method_classes(&mut registry);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "InheritedMethod",
        "callDeepInherited",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(10)));
}

#[test]
fn inherited_call_deep_overridden() {
    let mut registry = ClassRegistry::new();
    load_inherited_method_classes(&mut registry);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "InheritedMethod",
        "callDeepOverridden",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(2)));
}

// ---- Phase 15: string methods tests ----

#[test]
fn string_methods_substring() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("StringMethods.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StringMethods",
        "testSubstring",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(5)));
}

#[test]
fn string_methods_substring_range() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("StringMethods.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StringMethods",
        "testSubstringRange",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(5)));
}

#[test]
fn string_methods_indexof() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("StringMethods.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StringMethods",
        "testIndexOf",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(5)));
}

#[test]
fn string_methods_indexof_not_found() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("StringMethods.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StringMethods",
        "testIndexOfNotFound",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(-1)));
}

#[test]
fn string_methods_contains() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("StringMethods.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StringMethods",
        "testContains",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn string_methods_isempty() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("StringMethods.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StringMethods",
        "testIsEmpty",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn string_methods_compareto() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("StringMethods.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StringMethods",
        "testCompareTo",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn string_methods_startswith() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("StringMethods.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StringMethods",
        "testStartsWith",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn string_methods_endswith() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("StringMethods.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StringMethods",
        "testEndsWith",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn string_methods_trim() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("StringMethods.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StringMethods",
        "testTrim",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(2)));
}

#[test]
fn string_methods_tochararray() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("StringMethods.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StringMethods",
        "testToCharArray",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(131)));
}

// ---- Phase 15: main entry point tests ----

#[test]
fn main_hello_no_args() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("MainHello.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();

    // Build empty String[] array on the heap.
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 0);
    let main_args = vec![Slot::Reference(Some(arr_ref))];

    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "MainHello",
        "main",
        "([Ljava/lang/String;)V",
        &main_args,
    )
    .unwrap();
    assert_eq!(result, None);
    assert_eq!(String::from_utf8_lossy(&out), "no args\n");
}

#[test]
fn main_hello_with_args() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("MainHello.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();

    // Build String[] with ["Alice", "Bob"] on the heap.
    let alice_ref = heap.allocate_string("Alice".to_string());
    let bob_ref = heap.allocate_string("Bob".to_string());
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 2);
    heap.get_mut(arr_ref).unwrap().fields[0] = Slot::Reference(Some(alice_ref));
    heap.get_mut(arr_ref).unwrap().fields[1] = Slot::Reference(Some(bob_ref));
    let main_args = vec![Slot::Reference(Some(arr_ref))];

    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "MainHello",
        "main",
        "([Ljava/lang/String;)V",
        &main_args,
    )
    .unwrap();
    assert_eq!(result, None);
    assert_eq!(String::from_utf8_lossy(&out), "Alice\nBob\n");
}

// ---- Phase 16: PrintAll integration tests ----

fn load_print_all_class() -> ClassContext {
    let bytes = std::fs::read(fixture("PrintAll.class")).expect("PrintAll.class");
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

#[test]
fn print_all_long() {
    let ctx = load_print_all_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "PrintAll",
        "printLong",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
    assert!(
        String::from_utf8_lossy(&out).contains("9876543210"),
        "stdout should contain 9876543210, got: {}",
        String::from_utf8_lossy(&out)
    );
}

#[test]
fn print_all_double() {
    let ctx = load_print_all_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "PrintAll",
        "printDouble",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
    assert!(
        String::from_utf8_lossy(&out).contains("3.14"),
        "stdout should contain 3.14, got: {}",
        String::from_utf8_lossy(&out)
    );
}

#[test]
fn print_all_float() {
    let ctx = load_print_all_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "PrintAll",
        "printFloat",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
    assert!(
        String::from_utf8_lossy(&out).contains("2.5"),
        "stdout should contain 2.5, got: {}",
        String::from_utf8_lossy(&out)
    );
}

#[test]
fn print_all_boolean() {
    let ctx = load_print_all_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "PrintAll",
        "printBoolean",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
    let output = String::from_utf8_lossy(&out);
    assert!(
        output.contains("true") && output.contains("false"),
        "stdout should contain true and false, got: {output}",
    );
}

#[test]
fn print_all_char() {
    let ctx = load_print_all_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "PrintAll",
        "printChar",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
    assert!(
        String::from_utf8_lossy(&out).contains('Z'),
        "stdout should contain Z, got: {}",
        String::from_utf8_lossy(&out)
    );
}

#[test]
fn print_all_mixed() {
    let ctx = load_print_all_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "PrintAll",
        "printMixed",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
    assert_eq!(String::from_utf8_lossy(&out), "val=42\n");
}

// ---- Phase 16: ParseArgs integration tests ----

fn load_parse_args_class() -> ClassContext {
    let bytes = std::fs::read(fixture("ParseArgs.class")).expect("ParseArgs.class");
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

#[test]
fn parse_args_parse_int() {
    let ctx = load_parse_args_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    // Build String[] with ["123"]
    let s_ref = heap.allocate_string("123".to_string());
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 1);
    heap.get_mut(arr_ref).unwrap().fields[0] = Slot::Reference(Some(s_ref));
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ParseArgs",
        "parseInt",
        "([Ljava/lang/String;)I",
        &[Slot::Reference(Some(arr_ref))],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(123)));
}

#[test]
fn parse_args_add_parsed() {
    let ctx = load_parse_args_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    // Build String[] with ["10", "20"]
    let s1 = heap.allocate_string("10".to_string());
    let s2 = heap.allocate_string("20".to_string());
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 2);
    heap.get_mut(arr_ref).unwrap().fields[0] = Slot::Reference(Some(s1));
    heap.get_mut(arr_ref).unwrap().fields[1] = Slot::Reference(Some(s2));
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ParseArgs",
        "addParsed",
        "([Ljava/lang/String;)I",
        &[Slot::Reference(Some(arr_ref))],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(30)));
}

#[test]
fn parse_args_valueof() {
    let ctx = load_parse_args_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ParseArgs",
        "valueOf",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(42)));
}

#[test]
fn parse_args_math_max() {
    let ctx = load_parse_args_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ParseArgs",
        "mathMax",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(7)));
}

#[test]
fn parse_args_math_min() {
    let ctx = load_parse_args_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ParseArgs",
        "mathMin",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(3)));
}

#[test]
fn parse_args_math_abs() {
    let ctx = load_parse_args_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ParseArgs",
        "mathAbs",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(5)));
}

// ---- Phase 16: StringConcat integration tests ----

fn load_string_concat_class() -> ClassContext {
    let bytes = std::fs::read(fixture("StringConcat.class")).expect("StringConcat.class");
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

#[test]
fn string_concat_length() {
    let ctx = load_string_concat_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringConcat",
        "concatLength",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(10)));
}

#[test]
fn string_concat_bool_to_string() {
    let ctx = load_string_concat_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringConcat",
        "boolToString",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(4)));
}

#[test]
fn string_concat_long_to_string() {
    let ctx = load_string_concat_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringConcat",
        "longToString",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(3)));
}

#[test]
fn string_concat_char_to_string() {
    let ctx = load_string_concat_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringConcat",
        "charToString",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn string_concat_double_to_string() {
    let ctx = load_string_concat_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringConcat",
        "doubleToString",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

// ---- Phase 17: StringConcatFactory (invokedynamic) ----

fn load_string_concat_test_class() -> ClassContext {
    let bytes =
        std::fs::read(fixture("StringConcatTest.class")).expect("StringConcatTest.class");
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

#[test]
fn string_concat_simple() {
    let ctx = load_string_concat_test_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringConcatTest",
        "testSimple",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn string_concat_int() {
    let ctx = load_string_concat_test_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringConcatTest",
        "testInt",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn string_concat_chain() {
    let ctx = load_string_concat_test_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringConcatTest",
        "testChain",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn string_concat_boolean() {
    let ctx = load_string_concat_test_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringConcatTest",
        "testBoolean",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn string_concat_empty() {
    let ctx = load_string_concat_test_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringConcatTest",
        "testEmpty",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

// ---- Phase 17: LambdaMetafactory ----

fn load_lambda_test_class() -> ClassContext {
    let bytes = std::fs::read(fixture("LambdaTest.class")).expect("LambdaTest.class");
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

#[test]
fn lambda_simple_no_capture() {
    let ctx = load_lambda_test_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "LambdaTest",
        "testDouble",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(10)));
}

#[test]
fn lambda_with_capture() {
    let ctx = load_lambda_test_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "LambdaTest",
        "testCapture",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(107)));
}

#[test]
fn lambda_method_reference() {
    let ctx = load_lambda_test_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "LambdaTest",
        "testMethodRef",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(-42)));
}

#[test]
fn lambda_multi_capture() {
    let ctx = load_lambda_test_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "LambdaTest",
        "testMultiCapture",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(33)));
}

// ---- MonitorAndAbstract fixture ----

fn load_monitor_class() -> ClassContext {
    let bytes =
        std::fs::read(fixture("MonitorAndAbstract.class")).expect("MonitorAndAbstract.class");
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

#[test]
fn monitor_sync_block() {
    let ctx = load_monitor_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "MonitorAndAbstract",
        "syncBlock",
        "(I)I",
        &[Slot::Int(7)],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(14)));
}

#[test]
fn monitor_sync_method() {
    let ctx = load_monitor_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "MonitorAndAbstract",
        "syncMethod",
        "(I)I",
        &[Slot::Int(5)],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(15)));
}

#[test]
fn monitor_nested_sync() {
    let ctx = load_monitor_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "MonitorAndAbstract",
        "nestedSync",
        "(I)I",
        &[Slot::Int(10)],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(15)));
}

// ---- ExtendedMath integration tests ----

fn load_extended_math_class() -> ClassContext {
    let bytes = std::fs::read(fixture("ExtendedMath.class")).expect("ExtendedMath.class");
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

#[test]
fn extended_math_sqrt() {
    let ctx = load_extended_math_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ExtendedMath",
        "testSqrt",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn extended_math_pow() {
    let ctx = load_extended_math_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ExtendedMath",
        "testPow",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn extended_math_floor_ceil() {
    let ctx = load_extended_math_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ExtendedMath",
        "testFloorCeil",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn extended_math_round() {
    let ctx = load_extended_math_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ExtendedMath",
        "testRound",
        "()J",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Long(4)));
}

#[test]
fn extended_math_abs_long() {
    let ctx = load_extended_math_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ExtendedMath",
        "testAbsLong",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn extended_math_abs_double() {
    let ctx = load_extended_math_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ExtendedMath",
        "testAbsDouble",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn extended_math_max_long() {
    let ctx = load_extended_math_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ExtendedMath",
        "testMaxLong",
        "()J",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Long(200)));
}

#[test]
fn extended_math_min_long() {
    let ctx = load_extended_math_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ExtendedMath",
        "testMinLong",
        "()J",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Long(100)));
}

#[test]
fn extended_math_max_double() {
    let ctx = load_extended_math_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ExtendedMath",
        "testMaxDouble",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn extended_math_parse_long() {
    let ctx = load_extended_math_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ExtendedMath",
        "testParseLong",
        "()J",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Long(9_876_543_210)));
}

#[test]
fn extended_math_parse_double() {
    let ctx = load_extended_math_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ExtendedMath",
        "testParseDouble",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn extended_math_parse_float() {
    let ctx = load_extended_math_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ExtendedMath",
        "testParseFloat",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn extended_math_parse_boolean() {
    let ctx = load_extended_math_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ExtendedMath",
        "testParseBoolean",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn extended_math_long_valueof() {
    let ctx = load_extended_math_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ExtendedMath",
        "testLongValueOf",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn extended_math_constants() {
    let ctx = load_extended_math_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ExtendedMath",
        "testMathConstants",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

// ---- StringOps2 integration tests ----

fn load_string_ops2_class() -> ClassContext {
    let bytes = std::fs::read(fixture("StringOps2.class")).expect("StringOps2.class");
    let cf = duke_classfile::parse(&bytes).unwrap();
    build_class_context(&cf)
}

#[test]
fn string_ops2_to_upper_case() {
    let ctx = load_string_ops2_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringOps2",
        "testToUpperCase",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn string_ops2_to_lower_case() {
    let ctx = load_string_ops2_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringOps2",
        "testToLowerCase",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn string_ops2_replace_char() {
    let ctx = load_string_ops2_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringOps2",
        "testReplace",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn string_ops2_replace_string() {
    let ctx = load_string_ops2_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringOps2",
        "testReplaceString",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn string_ops2_split() {
    let ctx = load_string_ops2_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringOps2",
        "testSplit",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn string_ops2_hashcode() {
    let ctx = load_string_ops2_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringOps2",
        "testHashCode",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn string_ops2_tostring() {
    let ctx = load_string_ops2_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringOps2",
        "testToStringIdentity",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn string_ops2_replace_charsequence() {
    let ctx = load_string_ops2_class();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringOps2",
        "testReplaceCharSequence",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn class_literal_non_null() {
    assert_eq!(
        run_class_int("ClassLiteral.class", "testStringClass", "()I", vec![]),
        1
    );
}

#[test]
fn class_literal_self_ref() {
    assert_eq!(
        run_class_int("ClassLiteral.class", "testPrimitiveClass", "()I", vec![]),
        1
    );
}

#[test]
fn class_literal_interning() {
    assert_eq!(
        run_class_int("ClassLiteral.class", "testClassInterning", "()I", vec![]),
        1
    );
}

// ---- Phase 19: Enum integration tests ----

/// Helper that loads a class, calls `bootstrap_stdlib`, and runs a static method.
fn run_bootstrap_int(class_name: &str, method_name: &str, descriptor: &str) -> i32 {
    let ctx = load_class_context(class_name);
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    match execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        &entry_class,
        method_name,
        descriptor,
        &[],
    )
    .expect("execute_class failed")
    {
        Some(Slot::Int(v)) => v,
        other => panic!("unexpected result: {other:?}"),
    }
}

fn run_bootstrap_long(class_name: &str, method_name: &str, descriptor: &str) -> i64 {
    let ctx = load_class_context(class_name);
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut out,
        &entry_class,
        method_name,
        descriptor,
        &[],
    )
    .expect("fixture should execute");
    match result {
        Some(Slot::Long(value)) => value,
        other => panic!("expected long result, got {other:?}"),
    }
}

fn run_bootstrap_int_completion(class_name: &str, method_name: &str, descriptor: &str) -> i32 {
    let ctx = load_class_context(class_name);
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut out,
        &entry_class,
        method_name,
        descriptor,
        &[],
    )
    .expect("fixture should execute");
    match result {
        Some(Slot::Int(value)) => value,
        other => panic!("expected int result, got {other:?}"),
    }
}

fn run_bootstrap_with_string_args(
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[String],
) -> VmResult<Option<Slot>> {
    let ctx = load_class_context(class_name);
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let arg_slots: Vec<Slot> = args
        .iter()
        .map(|arg| Slot::Reference(Some(heap.allocate_string(arg.clone()))))
        .collect();
    let mut out: Vec<u8> = Vec::new();
    execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut out,
        &entry_class,
        method_name,
        descriptor,
        &arg_slots,
    )
}

fn run_bootstrap_with_output(
    class_name: &str,
    method_name: &str,
    descriptor: &str,
) -> VmResult<(Option<Slot>, Vec<String>)> {
    let ctx = load_class_context(class_name);
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut out,
        &entry_class,
        method_name,
        descriptor,
        &[],
    )?;
    let lines = String::from_utf8(out)
        .expect("captured output is utf8")
        .lines()
        .map(std::string::ToString::to_string)
        .collect();
    Ok((result, lines))
}

struct TempCleanup(std::path::PathBuf);

impl Drop for TempCleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn make_temp_root(prefix: &str) -> (std::path::PathBuf, TempCleanup) {
    let unique = format!(
        "{prefix}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is before UNIX_EPOCH")
            .as_nanos()
    );
    let root = std::env::temp_dir().join(unique);
    std::fs::create_dir_all(&root).expect("create temp root");
    let cleanup = TempCleanup(root.clone());
    (root, cleanup)
}

#[cfg(feature = "telemetry")]
fn run_fixture(
    class_name: &str,
    method_name: &str,
    descriptor: &str,
) -> (Option<Slot>, ClassRegistry) {
    let ctx = load_class_context(class_name);
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        &entry_class,
        method_name,
        descriptor,
        &[],
    )
    .expect("execute_class failed");
    (result, registry)
}

#[test]
fn enum_ordinal() {
    assert_eq!(
        run_bootstrap_int("SimpleEnum.class", "testOrdinal", "()I"),
        1
    );
}

#[test]
fn enum_name_length() {
    assert_eq!(run_bootstrap_int("SimpleEnum.class", "testName", "()I"), 3);
}

#[test]
fn enum_values_length() {
    assert_eq!(
        run_bootstrap_int("SimpleEnum.class", "testValues", "()I"),
        3
    );
}

#[test]
fn enum_valueof() {
    assert_eq!(
        run_bootstrap_int("SimpleEnum.class", "testValueOf", "()I"),
        2
    );
}

#[test]
fn enum_switch() {
    assert_eq!(
        run_bootstrap_int("SimpleEnum.class", "testSwitch", "()I"),
        20
    );
}

#[test]
fn enum_equality() {
    assert_eq!(
        run_bootstrap_int("SimpleEnum.class", "testEquality", "()I"),
        1
    );
}

// ---- StringBuilder tests ----

#[test]
fn sb_basic_append() {
    assert_eq!(
        run_bootstrap_int("StringBuilderTest.class", "testBasicAppend", "()I"),
        5
    );
}

#[test]
fn sb_chaining() {
    assert_eq!(
        run_bootstrap_int("StringBuilderTest.class", "testChaining", "()I"),
        6
    );
}

#[test]
fn sb_append_int() {
    assert_eq!(
        run_bootstrap_int("StringBuilderTest.class", "testAppendInt", "()I"),
        6
    );
}

#[test]
fn sb_append_long() {
    assert_eq!(
        run_bootstrap_int("StringBuilderTest.class", "testAppendLong", "()I"),
        3
    );
}

#[test]
fn sb_append_boolean() {
    assert_eq!(
        run_bootstrap_int("StringBuilderTest.class", "testAppendBoolean", "()I"),
        4
    );
}

#[test]
fn sb_append_char() {
    assert_eq!(
        run_bootstrap_int("StringBuilderTest.class", "testAppendChar", "()I"),
        1
    );
}

#[test]
fn sb_append_double() {
    assert_eq!(
        run_bootstrap_int("StringBuilderTest.class", "testAppendDouble", "()I"),
        4
    );
}

#[test]
fn sb_append_float() {
    assert_eq!(
        run_bootstrap_int("StringBuilderTest.class", "testAppendFloat", "()I"),
        3
    );
}

#[test]
fn sb_init_with_string() {
    assert_eq!(
        run_bootstrap_int("StringBuilderTest.class", "testInitWithString", "()I"),
        8
    );
}

#[test]
fn sb_length() {
    assert_eq!(
        run_bootstrap_int("StringBuilderTest.class", "testLength", "()I"),
        3
    );
}

#[test]
fn sb_loop_build() {
    assert_eq!(
        run_bootstrap_int("StringBuilderTest.class", "testLoopBuild", "()I"),
        5
    );
}

#[test]
fn sb_append_string_object() {
    assert_eq!(
        run_bootstrap_int("StringBuilderTest.class", "testAppendString", "()I"),
        5
    );
}

// ---- Character tests ----

#[test]
fn char_is_digit() {
    assert_eq!(
        run_bootstrap_int("CharacterTest.class", "testIsDigit", "()I"),
        1
    );
}

#[test]
fn char_is_digit_false() {
    assert_eq!(
        run_bootstrap_int("CharacterTest.class", "testIsDigitFalse", "()I"),
        1
    );
}

#[test]
fn char_is_letter() {
    assert_eq!(
        run_bootstrap_int("CharacterTest.class", "testIsLetter", "()I"),
        1
    );
}

#[test]
fn char_is_letter_false() {
    assert_eq!(
        run_bootstrap_int("CharacterTest.class", "testIsLetterFalse", "()I"),
        1
    );
}

#[test]
fn char_is_whitespace() {
    assert_eq!(
        run_bootstrap_int("CharacterTest.class", "testIsWhitespace", "()I"),
        1
    );
}

#[test]
fn char_is_uppercase() {
    assert_eq!(
        run_bootstrap_int("CharacterTest.class", "testIsUpperCase", "()I"),
        1
    );
}

#[test]
fn char_is_lowercase() {
    assert_eq!(
        run_bootstrap_int("CharacterTest.class", "testIsLowerCase", "()I"),
        1
    );
}

#[test]
fn char_to_uppercase() {
    assert_eq!(
        run_bootstrap_int("CharacterTest.class", "testToUpperCase", "()I"),
        65
    );
}

#[test]
fn char_to_lowercase() {
    assert_eq!(
        run_bootstrap_int("CharacterTest.class", "testToLowerCase", "()I"),
        97
    );
}

#[test]
fn char_is_letter_or_digit() {
    assert_eq!(
        run_bootstrap_int("CharacterTest.class", "testIsLetterOrDigit", "()I"),
        1
    );
}

#[test]
fn char_valueof_and_charvalue() {
    assert_eq!(
        run_bootstrap_int("CharacterTest.class", "testValueOf", "()I"),
        88
    );
}

#[test]
fn arraylist_size() {
    assert_eq!(
        run_bootstrap_int("ArrayListTest.class", "testSize", "()I"),
        3
    );
}

#[test]
fn arraylist_get() {
    assert_eq!(
        run_bootstrap_int("ArrayListTest.class", "testGet", "()I"),
        5
    );
}

#[test]
fn arraylist_foreach_count() {
    assert_eq!(
        run_bootstrap_int("ArrayListTest.class", "testForEachCount", "()I"),
        3
    );
}

#[test]
fn arraylist_foreach_sum() {
    assert_eq!(
        run_bootstrap_int("ArrayListTest.class", "testForEachSum", "()I"),
        8
    );
}

#[test]
fn arraylist_empty_foreach() {
    assert_eq!(
        run_bootstrap_int("ArrayListTest.class", "testEmptyForEach", "()I"),
        0
    );
}

#[test]
fn arraylist_single_element() {
    assert_eq!(
        run_bootstrap_int("ArrayListTest.class", "testSingleElement", "()I"),
        4
    );
}

#[test]
fn arraylist_add_returns_true() {
    assert_eq!(
        run_bootstrap_int("ArrayListTest.class", "testAddReturnsTrue", "()I"),
        1
    );
}

// ---- Phase 22: String.format() integration tests ----

#[test]
fn format_string() {
    assert_eq!(
        run_bootstrap_int("StringFormatTest.class", "testFormatString", "()I"),
        11
    );
}

#[test]
fn format_int() {
    assert_eq!(
        run_bootstrap_int("StringFormatTest.class", "testFormatInt", "()I"),
        2
    );
}

#[test]
fn format_multiple() {
    assert_eq!(
        run_bootstrap_int("StringFormatTest.class", "testFormatMultiple", "()I"),
        3
    );
}

#[test]
fn format_double() {
    assert_eq!(
        run_bootstrap_int("StringFormatTest.class", "testFormatDouble", "()I"),
        4
    );
}

#[test]
fn format_hex() {
    assert_eq!(
        run_bootstrap_int("StringFormatTest.class", "testFormatHex", "()I"),
        2
    );
}

#[test]
fn format_percent() {
    assert_eq!(
        run_bootstrap_int("StringFormatTest.class", "testFormatPercent", "()I"),
        4
    );
}

#[test]
fn format_null() {
    assert_eq!(
        run_bootstrap_int("StringFormatTest.class", "testFormatNull", "()I"),
        4
    );
}

#[test]
fn format_sum() {
    assert_eq!(
        run_bootstrap_int("StringFormatTest.class", "testFormatSum", "()I"),
        8
    );
}

// ---- Phase 22 Task 2: Arrays utilities + numeric constants ----

#[test]
fn arrays_fill_int() {
    assert_eq!(
        run_bootstrap_int("ArraysTest.class", "testFillInt", "()I"),
        14
    );
}

#[test]
fn arrays_copyof_truncate() {
    assert_eq!(
        run_bootstrap_int("ArraysTest.class", "testCopyOfTruncate", "()I"),
        3
    );
}

#[test]
fn arrays_copyof_extend() {
    assert_eq!(
        run_bootstrap_int("ArraysTest.class", "testCopyOfExtend", "()I"),
        0
    );
}

#[test]
fn arrays_sort_int() {
    assert_eq!(
        run_bootstrap_int("ArraysTest.class", "testSortInt", "()I"),
        19
    );
}

#[test]
fn integer_max_value() {
    assert_eq!(
        run_bootstrap_int("ArraysTest.class", "testIntegerMaxValue", "()I"),
        1
    );
}

#[test]
fn integer_min_value() {
    assert_eq!(
        run_bootstrap_int("ArraysTest.class", "testIntegerMinValue", "()I"),
        1
    );
}

#[test]
fn long_max_value() {
    assert_eq!(
        run_bootstrap_int("ArraysTest.class", "testLongMaxValue", "()I"),
        1
    );
}

#[test]
fn double_max_value() {
    assert_eq!(
        run_bootstrap_int("ArraysTest.class", "testDoubleMaxValue", "()I"),
        1
    );
}

#[test]
fn double_nan() {
    assert_eq!(
        run_bootstrap_int("ArraysTest.class", "testDoubleNaN", "()I"),
        1
    );
}

// ---- Phase 23 Task 1: HashMap ----

#[test]
fn hashmap_put_and_get() {
    assert_eq!(
        run_bootstrap_int("HashMapTest.class", "testPutAndGet", "()I"),
        30
    );
}

#[test]
fn hashmap_size() {
    assert_eq!(run_bootstrap_int("HashMapTest.class", "testSize", "()I"), 3);
}

#[test]
fn hashmap_contains_key() {
    assert_eq!(
        run_bootstrap_int("HashMapTest.class", "testContainsKey", "()I"),
        1
    );
}

#[test]
fn hashmap_get_missing() {
    assert_eq!(
        run_bootstrap_int("HashMapTest.class", "testGetMissing", "()I"),
        1
    );
}

#[test]
fn hashmap_remove() {
    assert_eq!(
        run_bootstrap_int("HashMapTest.class", "testRemove", "()I"),
        1
    );
}

#[test]
fn hashmap_is_empty() {
    assert_eq!(
        run_bootstrap_int("HashMapTest.class", "testIsEmpty", "()I"),
        1
    );
}

#[test]
fn hashmap_overwrite() {
    assert_eq!(
        run_bootstrap_int("HashMapTest.class", "testOverwrite", "()I"),
        1
    );
}

#[test]
fn hashmap_get_or_default() {
    assert_eq!(
        run_bootstrap_int("HashMapTest.class", "testGetOrDefault", "()I"),
        106
    );
}

#[test]
fn hashmap_overwrite_value_returns_new_value() {
    // kills 9488: fields[i+1]=val → fields[i]=val; get after overwrite returns null
    assert_eq!(
        run_bootstrap_int("HashMapTest.class", "testOverwriteValue", "()I"),
        99
    );
}

#[test]
fn hashmap_update_second_key_returns_new_value() {
    // kills 9491: i+=2 → i*=2; second key update is missed
    assert_eq!(
        run_bootstrap_int("HashMapTest.class", "testUpdateSecondKeyValue", "()I"),
        99
    );
}

#[test]
fn hashmap_remove_then_not_contains() {
    // kills 9586: truncate(len-2) → truncate(len+2); key persists after remove
    assert_eq!(
        run_bootstrap_int("HashMapTest.class", "testRemoveAndContains", "()I"),
        0
    );
}

#[test]
fn hashset_add_and_contains() {
    assert_eq!(
        run_bootstrap_int("HashSetTest.class", "testAddAndContains", "()I"),
        1
    );
}

#[test]
fn hashset_size() {
    assert_eq!(run_bootstrap_int("HashSetTest.class", "testSize", "()I"), 3);
}

#[test]
fn hashset_no_duplicates() {
    assert_eq!(
        run_bootstrap_int("HashSetTest.class", "testNoDuplicates", "()I"),
        2
    );
}

#[test]
fn hashset_remove() {
    assert_eq!(
        run_bootstrap_int("HashSetTest.class", "testRemove", "()I"),
        1
    );
}

#[test]
fn hashset_is_empty() {
    assert_eq!(
        run_bootstrap_int("HashSetTest.class", "testIsEmpty", "()I"),
        1
    );
}

#[test]
fn hashset_add_returns_false() {
    assert_eq!(
        run_bootstrap_int("HashSetTest.class", "testAddReturnsFalse", "()I"),
        1
    );
}

#[test]
fn frame_pool_does_not_change_fib_result() {
    // Regression guard: pool reuse must not corrupt frame state.
    // fib(25) = 75025 — stale locals between pool reuses would produce wrong answer.
    let result = run_bootstrap_int("BenchmarkSuite.class", "benchFib", "()I");
    assert_eq!(result, 75025);
}

#[test]
fn dispatch_cache_fib_correctness() {
    let result = run_bootstrap_int("BenchmarkSuite.class", "benchFib", "()I");
    assert_eq!(result, 75025);
}

#[test]
fn dispatch_cache_invokestatic_multiple_methods() {
    let sum = run_bootstrap_int("BenchmarkSuite.class", "benchSum", "()I");
    let fib = run_bootstrap_int("BenchmarkSuite.class", "benchFib", "()I");
    assert_eq!(fib, 75025);
    // benchSum overflows i32: sum(0..499999) = 124999750000 → wraps to 445698416
    assert_eq!(sum, 445_698_416_i32);
}

#[cfg(feature = "telemetry")]
#[test]
fn telemetry_bytecode_cost_counts_iadd() {
    // benchSum adds integers in a loop 0..500_000 (500_000 iadd ops).
    let (result, registry) = run_fixture("BenchmarkSuite.class", "benchSum", "()I");
    assert_eq!(result, Some(Slot::Int(445_698_416)));
    let stat = &registry.telemetry.bytecode_cost.by_opcode["iadd"];
    assert_eq!(stat.count, 500_000);
}

#[cfg(feature = "telemetry")]
#[test]
fn telemetry_object_lineage_records_allocations() {
    // ForEachTest creates an ArrayList and adds elements.
    let (_, registry) = run_fixture("ForEachTest.class", "main", "([Ljava/lang/String;)V");
    // At least one allocation site must exist (the ArrayList constructor).
    assert!(!registry.telemetry.object_lineage.sites.is_empty());
    // Verify some allocation is attributed to ForEachTest class.
    let has_foreach_alloc = registry
        .telemetry
        .object_lineage
        .sites
        .keys()
        .any(|(cls, _, _)| cls == "ForEachTest");
    assert!(
        has_foreach_alloc,
        "expected at least one allocation from ForEachTest"
    );
}

#[cfg(feature = "telemetry")]
#[test]
fn telemetry_native_boundary_records_println() {
    let (_, registry) = run_fixture("ForEachTest.class", "main", "([Ljava/lang/String;)V");
    // ForEachTest calls System.out.println which dispatches through println native.
    let stat = registry
        .telemetry
        .native_boundary
        .by_method
        .iter()
        .find(|((_, name), _)| name.contains("println"));
    assert!(
        stat.is_some(),
        "expected println to be recorded in native_boundary"
    );
    let (_, s) = stat.unwrap();
    assert!(s.calls > 0);
    assert_eq!(s.errors, 0);
}

#[cfg(feature = "telemetry")]
#[test]
fn telemetry_exception_flow_records_throw_and_catch() {
    let (result, registry) = run_fixture("ExceptionTest.class", "throwAndCatch", "()I");
    assert_eq!(result, Some(Slot::Int(42)));
    let events = &registry.telemetry.exception_flow.events;
    assert_eq!(events.len(), 1);
    assert!(events[0].exception_class.contains("RuntimeException"));
    assert!(events[0].catch_site.is_some(), "exception should be caught");
}

#[cfg(feature = "telemetry")]
#[test]
#[allow(clippy::len_zero)]
fn telemetry_exception_flow_rethrow_caught() {
    let (result, registry) = run_fixture("ExceptionTest.class", "rethrow", "()I");
    assert_eq!(result, Some(Slot::Int(99)));
    let events = &registry.telemetry.exception_flow.events;
    // Two throw events: inner throw + rethrow
    assert!(!events.is_empty());
    assert!(events.iter().all(|e| e.catch_site.is_some()));
}

#[cfg(feature = "telemetry")]
#[test]
fn telemetry_class_init_dag_records_clinit() {
    // ClinitTest has a static initializer that sets VALUE = 42.
    // Calling getValue() via invokestatic triggers ensure_initialized → <clinit>.
    let (result, registry) = run_fixture("ClinitTest.class", "getValue", "()I");
    assert_eq!(result, Some(Slot::Int(42)));
    let events = &registry.telemetry.class_init_dag.events;
    assert!(
        !events.is_empty(),
        "expected at least one class_init_dag event"
    );
    let ev = events
        .iter()
        .find(|e| e.class == "ClinitTest")
        .expect("expected ClinitTest clinit event");
    assert!(ev.duration_ns > 0, "clinit duration should be positive");
}

#[cfg(feature = "telemetry")]
#[test]
fn telemetry_dispatch_resolution_records_virtual_calls() {
    // ForEachTest calls invokevirtual on ArrayList (add) and invokeinterface
    // for the for-each iterator protocol (iterator, hasNext, next).
    let (_, registry) = run_fixture("ForEachTest.class", "main", "([Ljava/lang/String;)V");
    let dr = &registry.telemetry.dispatch_resolution;
    // At least some virtual/interface dispatch sites must have been recorded.
    assert!(
        !dr.by_site.is_empty(),
        "dispatch_resolution should have entries after ForEachTest"
    );
    // Every recorded site must have at least one call.
    for ((cls, cp), stat) in &dr.by_site {
        assert!(stat.calls > 0, "site {cls}[cp{cp}] should have calls > 0");
    }
}

// ---- Phase 24: GC stress tests ----

#[test]
fn gc_reclaims_short_lived_objects() {
    // GcStressTest allocates 2000 int[4] arrays in a loop.
    // Verify correct output sum = 0+1+...+1999 = 1999000.
    let result = run_bootstrap_int("GcStressTest.class", "run", "()I");
    assert_eq!(result, 1_999_000);
}

#[test]
fn gc_keeps_heap_bounded() {
    let loader = fixtures_loader();
    let bytes = loader.find_class("GcStressTest").unwrap();
    let cf = duke_classfile::parse(&bytes).unwrap();
    let ctx = build_class_context(&cf);
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let mut stdout = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut stdout,
        "GcStressTest",
        "run",
        "()I",
        &[],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1_999_000)));
    // 2000 arrays allocated; GC should have reclaimed most.
    // bootstrap_stdlib pre-populates ~200 permanent live objects (synthetic classes,
    // interned strings, static fields). After GC fires, the 2000 short-lived arrays
    // are collected, so total live count is dominated by bootstrap objects.
    // Without GC the heap would grow to 2000+ objects; with GC it stays bounded.
    let live = heap.len();
    assert!(
        live < 500,
        "heap has {live} live objects — GC may not have fired"
    );
}

#[test]
fn gc_generational_stress_test() {
    let ctx = load_class_context("GcGenerationalStressTest.class");
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    // Small young_capacity to force frequent minor GCs.
    heap.young_capacity = 32;

    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 1);
    heap.get_mut(arr_ref).unwrap().fields[0] = duke_runtime::Slot::Reference(None);

    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        &entry_class,
        "main",
        "([Ljava/lang/String;)V",
        &[duke_runtime::Slot::Reference(Some(arr_ref))],
    );
    assert!(
        result.is_ok(),
        "generational GC stress test failed: {result:?}"
    );
    let output = String::from_utf8(out).unwrap();
    // Verify long-lived objects survived all minor GCs.
    for i in 0..10 {
        assert!(
            output.contains(&format!("Survivor-{i}")),
            "long-lived object Survivor-{i} missing from output:\n{output}"
        );
    }
    // sum of "tmp-N".length() for N in 0..5000 = 38890
    assert!(
        output.contains("38890"),
        "expected sum 38890 in output:\n{output}"
    );
}

#[test]
fn callback_handler_is_dispatched_with_invoke_fn() {
    use std::sync::atomic::AtomicBool;
    static CALLED: AtomicBool = AtomicBool::new(false);

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    // Simple helper native: returns 42.
    registry.natives_mut().register(
        "duke/test/Helper",
        "answer",
        "()I",
        |_args, _heap, _out, _control| Ok(Some(Slot::Int(42))),
    );

    // Callback native: invokes the helper and returns its result.
    registry.natives_mut().register_callback(
        "duke/test/Caller",
        "call",
        "()I",
        |_args, heap, output, _control, ops| {
            CALLED.store(true, std::sync::atomic::Ordering::SeqCst);
            ops.invoke(heap, output, "duke/test/Helper", "answer", "()I", vec![])
        },
    );

    let loader = duke_loader::DirectoryLoader::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures"),
    );
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "duke/test/Caller",
        "call",
        "()I",
        &[],
    );
    assert!(result.is_ok(), "callback dispatch failed: {result:?}");
    assert_eq!(result.unwrap(), Some(Slot::Int(42)));
    assert!(
        CALLED.load(std::sync::atomic::Ordering::SeqCst),
        "callback handler was never invoked"
    );
}

#[test]
fn interpreter_callback_ops_can_invoke_registered_lambda_classes() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.natives_mut().register(
        "duke/test/LambdaHelper",
        "answer",
        "()I",
        |_args, _heap, _out, _control| Ok(Some(Slot::Int(42))),
    );

    let lambda_class = registry.register_lambda(LambdaInfo {
        impl_class: "duke/test/LambdaHelper".to_string(),
        impl_method: "answer".to_string(),
        impl_desc: "()I".to_string(),
        impl_kind: 6,
        sam_method: "getAsInt".to_string(),
        sam_desc: "()I".to_string(),
        captured_count: 0,
    });
    let lambda_ref = heap.allocate(lambda_class.clone(), 0);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let mut ops = InterpreterCallbackOps {
        registry: &mut registry,
        loader: &loader,
    };

    let result = ops
        .invoke(
            &mut heap,
            &mut out,
            &lambda_class,
            "getAsInt",
            "()I",
            vec![Slot::Reference(Some(lambda_ref))],
        )
        .expect("lambda invoke should succeed");
    assert_eq!(result, Some(Slot::Int(42)));
}

#[test]
fn interpreter_callback_ops_can_invoke_registered_virtual_lambda_classes() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.natives_mut().register(
        "duke/test/LambdaTarget",
        "increment",
        "(I)I",
        |args, _heap, _out, _control| match args {
            [Slot::Reference(Some(_)), Slot::Int(value)] => Ok(Some(Slot::Int(value + 1))),
            _ => Err(VmError::TypeMismatch {
                expected: "reference,int",
                got: "other",
            }),
        },
    );

    let lambda_class = registry.register_lambda(LambdaInfo {
        impl_class: "duke/test/LambdaTarget".to_string(),
        impl_method: "increment".to_string(),
        impl_desc: "(I)I".to_string(),
        impl_kind: 5,
        sam_method: "applyAsInt".to_string(),
        sam_desc: "(I)I".to_string(),
        captured_count: 1,
    });
    let target_ref = heap.allocate("duke/test/LambdaTarget".to_string(), 0);
    let lambda_ref = heap.allocate(lambda_class.clone(), 1);
    heap.get_mut(lambda_ref).unwrap().fields[0] = Slot::Reference(Some(target_ref));
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let mut ops = InterpreterCallbackOps {
        registry: &mut registry,
        loader: &loader,
    };

    let result = ops
        .invoke(
            &mut heap,
            &mut out,
            &lambda_class,
            "applyAsInt",
            "(I)I",
            vec![Slot::Reference(Some(lambda_ref)), Slot::Int(41)],
        )
        .expect("lambda invoke should succeed");
    assert_eq!(result, Some(Slot::Int(42)));
}

// ---- Bytecode-level Callback dispatch tests (Sites 2-4) ----
//
// These tests verify that `HandlerKind::Callback` handlers fire when the
// call site is reached via *bytecode* (invokestatic / invokevirtual /
// invokeinterface), not just via the top-level fast-path.
//
// Pattern: bootstrap stdlib (so the fixture class can run), then
// *override* one specific native with a Callback handler, run the fixture
// bytecode, and assert both the result and the CALLED flag.

/// Site 2 — invokestatic Callback arm.
///
/// `ParseArgs.parseInt()` bytecode contains:
///   `invokestatic java/lang/Integer.parseInt:(Ljava/lang/String;)I`
/// We override that registration with a Callback handler that delegates to
/// `invoke`, proving the arm wires the closure correctly end-to-end.
#[test]
fn callback_fires_via_invokestatic_bytecode() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static CALLED: AtomicBool = AtomicBool::new(false);

    let ctx = load_class_context("ParseArgs.class");
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    // Override the Simple Integer.parseInt with a Callback that records
    // invocation and delegates via `invoke` back to the (already-registered)
    // helper that bootstrap_stdlib set up as a Simple handler on
    // "java/lang/Integer"/"parseInt".
    // Because we overwrite the key the Simple handler is gone — we compute
    // the parse directly inside the callback instead.
    registry.natives_mut().register_callback(
        "java/lang/Integer",
        "parseInt",
        "(Ljava/lang/String;)I",
        |args, heap, _output, _control, _invoke| {
            CALLED.store(true, Ordering::SeqCst);
            // args[0] is the String reference; extract its string_value.
            let s = match &args[0] {
                Slot::Reference(Some(r)) => heap
                    .get(*r)
                    .ok()
                    .and_then(|o| o.string_value.clone())
                    .unwrap_or_default(),
                _ => return Err(VmError::NullPointerException),
            };
            let n: i32 = s.parse().map_err(|_| VmError::NullPointerException)?;
            Ok(Some(Slot::Int(n)))
        },
    );

    let loader = fixtures_loader();
    // Build String[] = ["123"] for ParseArgs.parseInt
    let s_ref = heap.allocate_string("123".to_string());
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 1);
    heap.get_mut(arr_ref).unwrap().fields[0] = Slot::Reference(Some(s_ref));
    let mut out: Vec<u8> = Vec::new();

    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ParseArgs",
        "parseInt",
        "([Ljava/lang/String;)I",
        &[Slot::Reference(Some(arr_ref))],
    );
    assert!(
        result.is_ok(),
        "invokestatic Callback dispatch failed: {result:?}"
    );
    assert_eq!(result.unwrap(), Some(Slot::Int(123)));
    assert!(
        CALLED.load(Ordering::SeqCst),
        "Callback handler was never invoked via invokestatic bytecode"
    );
}

/// Site 3 — invokevirtual Callback arm.
///
/// `ParseArgs.valueOf()` bytecode contains:
///   `invokevirtual java/lang/Integer.intValue:()I`
/// We override that registration with a Callback handler.
#[test]
fn callback_fires_via_invokevirtual_bytecode() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static CALLED: AtomicBool = AtomicBool::new(false);

    let ctx = load_class_context("ParseArgs.class");
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    // Override Integer.intValue with a Callback.
    // The Integer heap object stores the boxed int in fields[0].
    registry.natives_mut().register_callback(
        "java/lang/Integer",
        "intValue",
        "()I",
        |args, heap, _output, _control, _invoke| {
            CALLED.store(true, Ordering::SeqCst);
            // args[0] is `this` (the Integer object); fields[0] holds the int.
            let Slot::Reference(Some(r)) = &args[0] else {
                return Err(VmError::NullPointerException);
            };
            let val = heap.get(*r)?.fields[0];
            Ok(Some(val))
        },
    );

    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();

    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ParseArgs",
        "valueOf",
        "()I",
        &[],
    );
    assert!(
        result.is_ok(),
        "invokevirtual Callback dispatch failed: {result:?}"
    );
    assert_eq!(result.unwrap(), Some(Slot::Int(42)));
    assert!(
        CALLED.load(Ordering::SeqCst),
        "Callback handler was never invoked via invokevirtual bytecode"
    );
}

/// Site 4 — invokeinterface Callback arm.
///
/// `ArrayListTest.testForEachCount()` bytecode uses:
///   `invokeinterface java/util/Iterator.hasNext:()Z`
/// dispatched on the actual runtime class `duke/util/ArrayListIterator`.
/// We override `duke/util/ArrayListIterator.hasNext` with a Callback that
/// immediately returns false (0), making the for-each body not execute and
/// the count stay at 0.  This verifies the invokeinterface Callback arm
/// fires.
#[test]
fn callback_fires_via_invokeinterface_bytecode() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static CALLED: AtomicBool = AtomicBool::new(false);

    let ctx = load_class_context("ArrayListTest.class");
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    // Override ArrayListIterator.hasNext with a Callback that records
    // invocation and immediately signals "no more elements" (returns false).
    registry.natives_mut().register_callback(
        "duke/util/ArrayListIterator",
        "hasNext",
        "()Z",
        |_args, _heap, _output, _control, _invoke| {
            CALLED.store(true, Ordering::SeqCst);
            Ok(Some(Slot::Int(0))) // false — loop body never runs
        },
    );

    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();

    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "ArrayListTest",
        "testForEachCount",
        "()I",
        &[],
    );
    assert!(
        result.is_ok(),
        "invokeinterface Callback dispatch failed: {result:?}"
    );
    // hasNext always returns false → loop body never runs → count = 0.
    assert_eq!(result.unwrap(), Some(Slot::Int(0)));
    assert!(
        CALLED.load(Ordering::SeqCst),
        "Callback handler was never invoked via invokeinterface bytecode"
    );
}

/// Site 5 — lambda SAM virtual/interface native-fallback Callback arm.
///
/// `LambdaCallbackTest.capturedLengthViaMethodRef("hello")` compiles to:
///
///   invokedynamic … get:(Ljava/lang/String;)LLambdaCallbackTest$IntSupplier;
///   // creates `$$Lambda$0` with `impl_class`="java/lang/String",
///   //   `impl_method`="length", `impl_kind`=5 (`REF_invokeVirtual`),
///   //   `captured_count`=1 (the string "hello")
///   invokeinterface LambdaCallbackTest$IntSupplier.get:()I
///   // → lambda SAM: `impl_kind`==5, `resolve_method_in_hierarchy` returns None
///   //   (String has no bytecode methods in Duke), so falls to Site 5:
///   //   `registry.natives_mut().get_kind`("java/lang/String", "length", "()I")
///
/// We override `String.length` with a Callback handler to prove the arm fires.
#[test]
fn callback_fires_via_lambda_sam_fallback() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static CALLED: AtomicBool = AtomicBool::new(false);

    let ctx = load_class_context("LambdaCallbackTest.class");
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    // javac emits `dup; invokestatic Objects.requireNonNull; pop` for
    // captured instance method references.  Register a ClassContext and a
    // passthrough native so the invokestatic dispatch doesn't fail.
    registry.register(ClassContext {
        class_name: "java/util/Objects".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    });
    registry.natives_mut().register(
        "java/util/Objects",
        "requireNonNull",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        |args, _heap, _out, _control| Ok(Some(args[0])),
    );

    // Override String.length with a Callback.  This replaces the Simple
    // handler that bootstrap_stdlib registered, so the lambda SAM fallback
    // (Site 5) must route through the Callback arm to fire at all.
    registry.natives_mut().register_callback(
        "java/lang/String",
        "length",
        "()I",
        |args, heap, _output, _control, _invoke| {
            CALLED.store(true, Ordering::SeqCst);
            // args[0] is `this` (the captured String reference).
            let Slot::Reference(Some(r)) = &args[0] else {
                return Err(VmError::NullPointerException);
            };
            let len = i32::try_from(heap.get(*r)?.string_value.as_deref().unwrap_or("").len())
                .unwrap_or(i32::MAX);
            Ok(Some(Slot::Int(len)))
        },
    );

    let loader = fixtures_loader();
    let s_ref = heap.allocate_string("hello".to_string());
    let mut out: Vec<u8> = Vec::new();

    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "LambdaCallbackTest",
        "capturedLengthViaMethodRef",
        "(Ljava/lang/String;)I",
        &[Slot::Reference(Some(s_ref))],
    );
    assert!(
        result.is_ok(),
        "lambda SAM Callback dispatch failed: {result:?}"
    );
    assert_eq!(result.unwrap(), Some(Slot::Int(5)));
    assert!(
        CALLED.load(Ordering::SeqCst),
        "Callback handler was never invoked via lambda SAM fallback (Site 5)"
    );
}

#[test]
fn integer_compare_to_less_returns_negative() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let a = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(a).unwrap().fields[0] = Slot::Int(3);
    let b = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(b).unwrap().fields[0] = Slot::Int(5);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "java/lang/Integer",
        "compareTo",
        "(Ljava/lang/Object;)I",
        &[Slot::Reference(Some(a)), Slot::Reference(Some(b))],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(-1)));
}

#[test]
fn integer_compare_to_equal_returns_zero() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let a = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(a).unwrap().fields[0] = Slot::Int(7);
    let b = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(b).unwrap().fields[0] = Slot::Int(7);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "java/lang/Integer",
        "compareTo",
        "(Ljava/lang/Object;)I",
        &[Slot::Reference(Some(a)), Slot::Reference(Some(b))],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(0)));
}

#[test]
fn string_compare_to_apple_less_than_banana() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let a = heap.allocate_string("apple".to_string());
    let b = heap.allocate_string("banana".to_string());
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "java/lang/String",
        "compareTo",
        "(Ljava/lang/Object;)I",
        &[Slot::Reference(Some(a)), Slot::Reference(Some(b))],
    )
    .unwrap();
    match result {
        Some(Slot::Int(n)) => assert!(n < 0, "apple < banana: expected negative, got {n}"),
        other => panic!("expected Int, got {other:?}"),
    }
}

#[test]
fn integer_compare_to_greater_returns_positive() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let a = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(a).unwrap().fields[0] = Slot::Int(9);
    let b = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(b).unwrap().fields[0] = Slot::Int(3);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "java/lang/Integer",
        "compareTo",
        "(Ljava/lang/Object;)I",
        &[Slot::Reference(Some(a)), Slot::Reference(Some(b))],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn long_compare_to_less_returns_negative() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let a = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(a).unwrap().fields[0] = Slot::Long(100);
    let b = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(b).unwrap().fields[0] = Slot::Long(200);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "java/lang/Long",
        "compareTo",
        "(Ljava/lang/Object;)I",
        &[Slot::Reference(Some(a)), Slot::Reference(Some(b))],
    )
    .unwrap();
    assert_eq!(result, Some(Slot::Int(-1)));
}

#[test]
fn double_compare_to_nan_is_greatest() {
    // Java spec: NaN > any value including POSITIVE_INFINITY.
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();

    // Case 1: NaN > +∞ → result should be 1.
    let nan_ref = heap.allocate("java/lang/Double".to_string(), 1);
    heap.get_mut(nan_ref).unwrap().fields[0] = Slot::Double(f64::NAN);
    let inf_ref = heap.allocate("java/lang/Double".to_string(), 1);
    heap.get_mut(inf_ref).unwrap().fields[0] = Slot::Double(f64::INFINITY);
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "java/lang/Double",
        "compareTo",
        "(Ljava/lang/Object;)I",
        &[
            Slot::Reference(Some(nan_ref)),
            Slot::Reference(Some(inf_ref)),
        ],
    )
    .unwrap();
    assert_eq!(
        result,
        Some(Slot::Int(1)),
        "NaN.compareTo(+Inf) should be 1 (NaN is greatest)"
    );

    // Case 2: 1.0 < NaN → result should be -1.
    let one_ref = heap.allocate("java/lang/Double".to_string(), 1);
    heap.get_mut(one_ref).unwrap().fields[0] = Slot::Double(1.0);
    let nan_ref2 = heap.allocate("java/lang/Double".to_string(), 1);
    heap.get_mut(nan_ref2).unwrap().fields[0] = Slot::Double(f64::NAN);
    let mut out2: Vec<u8> = Vec::new();
    let result2 = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out2,
        "java/lang/Double",
        "compareTo",
        "(Ljava/lang/Object;)I",
        &[
            Slot::Reference(Some(one_ref)),
            Slot::Reference(Some(nan_ref2)),
        ],
    )
    .unwrap();
    assert_eq!(
        result2,
        Some(Slot::Int(-1)),
        "1.0.compareTo(NaN) should be -1 (NaN is greatest)"
    );
}

// ---- Phase 26 Task 4: ArrayList.sort(Comparator) via CallbackNativeHandler ----

#[test]
fn array_list_sort_integers_via_callback() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    // Build ArrayList [Integer(3), Integer(1), Integer(4)]
    let list = heap.allocate("java/util/ArrayList".to_string(), 4);
    heap.get_mut(list).unwrap().fields[0] = Slot::Int(3); // size
    let make_int = |heap: &mut duke_gc::Heap, n: i32| -> u64 {
        let r = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Int(n);
        r
    };
    let i3 = make_int(&mut heap, 3);
    let i1 = make_int(&mut heap, 1);
    let i4 = make_int(&mut heap, 4);
    heap.get_mut(list).unwrap().fields[1] = Slot::Reference(Some(i3));
    heap.get_mut(list).unwrap().fields[2] = Slot::Reference(Some(i1));
    heap.get_mut(list).unwrap().fields[3] = Slot::Reference(Some(i4));

    let loader = duke_loader::DirectoryLoader::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures"),
    );
    let mut out: Vec<u8> = Vec::new();
    execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "java/util/ArrayList",
        "sort",
        "(Ljava/util/Comparator;)V",
        &[Slot::Reference(Some(list)), Slot::Reference(None)], // null Comparator
    )
    .unwrap();

    // After sort: fields[1..=3] = Integer(1), Integer(3), Integer(4)
    let val = |heap: &duke_gc::Heap, s: &Slot| -> i32 {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r).unwrap().fields.first() {
                Some(Slot::Int(n)) => *n,
                _ => -1,
            },
            _ => -1,
        }
    };
    let f = |i: usize| heap.get(list).unwrap().fields[i];
    assert_eq!(val(&heap, &f(1)), 1);
    assert_eq!(val(&heap, &f(2)), 3);
    assert_eq!(val(&heap, &f(3)), 4);
}

#[test]
fn array_list_sort_strings_via_callback() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let list = heap.allocate("java/util/ArrayList".to_string(), 4);
    heap.get_mut(list).unwrap().fields[0] = Slot::Int(3);
    let sb = heap.allocate_string("banana".to_string());
    let sa = heap.allocate_string("apple".to_string());
    let sc = heap.allocate_string("cherry".to_string());
    heap.get_mut(list).unwrap().fields[1] = Slot::Reference(Some(sb));
    heap.get_mut(list).unwrap().fields[2] = Slot::Reference(Some(sa));
    heap.get_mut(list).unwrap().fields[3] = Slot::Reference(Some(sc));

    let loader = duke_loader::DirectoryLoader::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures"),
    );
    let mut out: Vec<u8> = Vec::new();
    execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "java/util/ArrayList",
        "sort",
        "(Ljava/util/Comparator;)V",
        &[Slot::Reference(Some(list)), Slot::Reference(None)],
    )
    .unwrap();

    let str_val = |heap: &duke_gc::Heap, s: &Slot| -> String {
        match s {
            Slot::Reference(Some(r)) => heap
                .get(*r)
                .unwrap()
                .string_value
                .clone()
                .unwrap_or_default(),
            _ => String::new(),
        }
    };
    let f = |i: usize| heap.get(list).unwrap().fields[i];
    assert_eq!(str_val(&heap, &f(1)), "apple");
    assert_eq!(str_val(&heap, &f(2)), "banana");
    assert_eq!(str_val(&heap, &f(3)), "cherry");
}

// Fix 6: boundary tests for array_list_sort

#[test]
fn array_list_sort_empty_list_is_noop() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    // ArrayList with size=0 — allocate just the size field slot.
    let list = heap.allocate("java/util/ArrayList".to_string(), 1);
    heap.get_mut(list).unwrap().fields[0] = Slot::Int(0);

    let loader = duke_loader::DirectoryLoader::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures"),
    );
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "java/util/ArrayList",
        "sort",
        "(Ljava/util/Comparator;)V",
        &[Slot::Reference(Some(list)), Slot::Reference(None)],
    );
    assert!(result.is_ok(), "empty sort should not error: {result:?}");
    assert_eq!(result.unwrap(), None);
    // Size field still 0.
    assert_eq!(heap.get(list).unwrap().fields[0], Slot::Int(0));
}

#[test]
fn array_list_sort_single_element_is_noop() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let list = heap.allocate("java/util/ArrayList".to_string(), 2);
    heap.get_mut(list).unwrap().fields[0] = Slot::Int(1);
    let elem = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(elem).unwrap().fields[0] = Slot::Int(42);
    heap.get_mut(list).unwrap().fields[1] = Slot::Reference(Some(elem));

    let loader = duke_loader::DirectoryLoader::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures"),
    );
    let mut out: Vec<u8> = Vec::new();
    execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "java/util/ArrayList",
        "sort",
        "(Ljava/util/Comparator;)V",
        &[Slot::Reference(Some(list)), Slot::Reference(None)],
    )
    .unwrap();

    // Single element unchanged.
    match &heap.get(list).unwrap().fields[1] {
        Slot::Reference(Some(r)) => {
            let r = *r;
            assert_eq!(heap.get(r).unwrap().fields[0], Slot::Int(42));
        }
        other => panic!("unexpected slot: {other:?}"),
    }
}

#[test]
fn array_list_sort_already_sorted_unchanged() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let list = heap.allocate("java/util/ArrayList".to_string(), 4);
    heap.get_mut(list).unwrap().fields[0] = Slot::Int(3);
    let make_int = |heap: &mut duke_gc::Heap, n: i32| -> u64 {
        let r = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Int(n);
        r
    };
    let i1 = make_int(&mut heap, 1);
    let i2 = make_int(&mut heap, 2);
    let i3 = make_int(&mut heap, 3);
    heap.get_mut(list).unwrap().fields[1] = Slot::Reference(Some(i1));
    heap.get_mut(list).unwrap().fields[2] = Slot::Reference(Some(i2));
    heap.get_mut(list).unwrap().fields[3] = Slot::Reference(Some(i3));

    let loader = duke_loader::DirectoryLoader::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures"),
    );
    let mut out: Vec<u8> = Vec::new();
    execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "java/util/ArrayList",
        "sort",
        "(Ljava/util/Comparator;)V",
        &[Slot::Reference(Some(list)), Slot::Reference(None)],
    )
    .unwrap();

    let int_val = |heap: &duke_gc::Heap, i: usize| -> i32 {
        match heap.get(list).unwrap().fields[i] {
            Slot::Reference(Some(r)) => match heap.get(r).unwrap().fields[0] {
                Slot::Int(n) => n,
                _ => -1,
            },
            _ => -1,
        }
    };
    assert_eq!(int_val(&heap, 1), 1);
    assert_eq!(int_val(&heap, 2), 2);
    assert_eq!(int_val(&heap, 3), 3);
}

#[test]
fn array_list_sort_duplicates() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    // [3, 1, 1, 2] → [1, 1, 2, 3]
    let list = heap.allocate("java/util/ArrayList".to_string(), 5);
    heap.get_mut(list).unwrap().fields[0] = Slot::Int(4);
    let make_int = |heap: &mut duke_gc::Heap, n: i32| -> u64 {
        let r = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Int(n);
        r
    };
    let r3 = make_int(&mut heap, 3);
    let r1a = make_int(&mut heap, 1);
    let r1b = make_int(&mut heap, 1);
    let r2 = make_int(&mut heap, 2);
    heap.get_mut(list).unwrap().fields[1] = Slot::Reference(Some(r3));
    heap.get_mut(list).unwrap().fields[2] = Slot::Reference(Some(r1a));
    heap.get_mut(list).unwrap().fields[3] = Slot::Reference(Some(r1b));
    heap.get_mut(list).unwrap().fields[4] = Slot::Reference(Some(r2));

    let loader = duke_loader::DirectoryLoader::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures"),
    );
    let mut out: Vec<u8> = Vec::new();
    execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "java/util/ArrayList",
        "sort",
        "(Ljava/util/Comparator;)V",
        &[Slot::Reference(Some(list)), Slot::Reference(None)],
    )
    .unwrap();

    let int_val = |heap: &duke_gc::Heap, i: usize| -> i32 {
        match heap.get(list).unwrap().fields[i] {
            Slot::Reference(Some(r)) => match heap.get(r).unwrap().fields[0] {
                Slot::Int(n) => n,
                _ => -1,
            },
            _ => -1,
        }
    };
    assert_eq!(int_val(&heap, 1), 1);
    assert_eq!(int_val(&heap, 2), 1);
    assert_eq!(int_val(&heap, 3), 2);
    assert_eq!(int_val(&heap, 4), 3);
}

// ---- Phase 26 Task 5: CollectionsSortTest end-to-end integration test ----

#[test]
fn collections_sort_end_to_end() {
    let ctx = load_class_context("CollectionsSortTest.class");
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let loader = fixtures_loader();
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 1);
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        &entry_class,
        "main",
        "([Ljava/lang/String;)V",
        &[Slot::Reference(Some(arr_ref))],
    );
    assert!(result.is_ok(), "CollectionsSortTest failed: {result:?}");
    let output = String::from_utf8(out).unwrap();
    // Integer sort: 1 1 3 4 5 (one per line)
    assert!(
        output.contains("1\n1\n3\n4\n5"),
        "integer sort wrong:\n{output}"
    );
    // String sort: apple banana cherry (one per line)
    assert!(
        output.contains("apple\nbanana\ncherry"),
        "string sort wrong:\n{output}"
    );
}

#[test]
fn collections_sort_null_list_raises_npe() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "java/util/Collections",
        "sort",
        "(Ljava/util/List;)V",
        &[Slot::Reference(None)],
    );
    assert!(
        matches!(result, Err(VmError::NullPointerException)),
        "expected NullPointerException, got {result:?}"
    );
}

#[test]
fn collections_sort_empty_list_is_noop() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    // Empty ArrayList: size=0
    let list = heap.allocate("java/util/ArrayList".to_string(), 1);
    heap.get_mut(list).unwrap().fields[0] = Slot::Int(0);
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "java/util/Collections",
        "sort",
        "(Ljava/util/List;)V",
        &[Slot::Reference(Some(list))],
    );
    assert!(result.is_ok(), "empty list sort failed: {result:?}");
}

// --- Phase 27: try-with-resources ---

#[test]
fn try_with_resources_simple_value() {
    assert_eq!(
        run_bootstrap_int("TryWithResources.class", "simpleValue", "()I"),
        42
    );
}

#[test]
fn try_with_resources_closed_on_success() {
    assert_eq!(
        run_bootstrap_int("TryWithResources.class", "closedOnSuccess", "()I"),
        1
    );
}

#[test]
fn try_with_resources_closed_on_exception() {
    assert_eq!(
        run_bootstrap_int("TryWithResources.class", "closedOnException", "()I"),
        1
    );
}

#[test]
fn try_with_resources_nested_closed() {
    assert_eq!(
        run_bootstrap_int("TryWithResources.class", "nestedClosed", "()I"),
        2
    );
}

// ---- Phase 28: File I/O metadata ----

#[test]
fn file_io_metadata_reports_file_and_directory_kinds() {
    let (root, _cleanup) = make_temp_root("duke-file-io");

    let file_path = root.join("sample.txt");
    let dir_path = root.join("nested");
    let missing_path = root.join("missing.txt");
    std::fs::write(&file_path, b"abc").expect("write temp file");
    std::fs::create_dir_all(&dir_path).expect("create temp dir");

    let result = run_bootstrap_with_string_args(
        "FileIoTest.class",
        "inspectKinds",
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
        &[
            file_path.to_string_lossy().into_owned(),
            dir_path.to_string_lossy().into_owned(),
            missing_path.to_string_lossy().into_owned(),
        ],
    );

    assert!(
        result.is_ok(),
        "expected File metadata support; current Duke failed with {result:?}"
    );
    let Some(Slot::Int(mask)) = result.unwrap() else {
        panic!("expected int bitmask result");
    };
    assert_ne!(mask & 1, 0, "existing file should report exists()");
    assert_ne!(mask & 2, 0, "existing file should report isFile()");
    assert_eq!(mask & 4, 0, "existing file should not report isDirectory()");
    assert_ne!(mask & 8, 0, "existing directory should report exists()");
    assert_eq!(
        mask & 16,
        0,
        "existing directory should not report isFile()"
    );
    assert_ne!(
        mask & 32,
        0,
        "existing directory should report isDirectory()"
    );
    assert_eq!(mask & 64, 0, "missing path should not report exists()");
    assert_eq!(mask & 128, 0, "missing path should not report isFile()");
    assert_eq!(
        mask & 256,
        0,
        "missing path should not report isDirectory()"
    );
}

#[test]
fn file_io_reads_all_bytes_and_sums_them() {
    let (root, _cleanup) = make_temp_root("duke-file-io-read");
    let input_path = root.join("bytes.bin");
    std::fs::write(&input_path, [1_u8, 2, 3, 4]).expect("write input file");

    let result = run_bootstrap_with_string_args(
        "FileIoTest.class",
        "readAllAndSum",
        "(Ljava/lang/String;)I",
        &[input_path.to_string_lossy().into_owned()],
    );

    assert!(
        result.is_ok(),
        "expected FileInputStream read support; current Duke failed with {result:?}"
    );
    assert_eq!(
        result.unwrap(),
        Some(Slot::Int(10)),
        "readAllAndSum should return the sum of all input bytes"
    );
}

#[test]
fn file_io_copies_bytes_via_try_with_resources() {
    let (root, _cleanup) = make_temp_root("duke-file-io-copy");
    let input_path = root.join("input.bin");
    let output_path = root.join("output.bin");
    let input_bytes = [9_u8, 8, 7, 6];
    std::fs::write(&input_path, input_bytes).expect("write input file");

    let result = run_bootstrap_with_string_args(
        "FileIoTest.class",
        "copyAndCount",
        "(Ljava/lang/String;Ljava/lang/String;)I",
        &[
            input_path.to_string_lossy().into_owned(),
            output_path.to_string_lossy().into_owned(),
        ],
    );

    assert!(
        result.is_ok(),
        "expected FileOutputStream write support; current Duke failed with {result:?}"
    );
    assert_eq!(
        result.unwrap(),
        Some(Slot::Int(4)),
        "copyAndCount should report the number of copied bytes"
    );
    assert_eq!(
        std::fs::read(&output_path).expect("read output file"),
        input_bytes,
        "copied output bytes should match the input bytes"
    );
}

#[test]
fn file_io_copies_bytes_with_block_read_and_write() {
    let (root, _cleanup) = make_temp_root("duke-file-io-buffer");
    let input_path = root.join("input.bin");
    let output_path = root.join("output.bin");
    let input_bytes = [5_u8, 4, 3, 2];
    std::fs::write(&input_path, input_bytes).expect("write input file");

    let result = run_bootstrap_with_string_args(
        "FileIoTest.class",
        "copyWithBuffer",
        "(Ljava/lang/String;Ljava/lang/String;)I",
        &[
            input_path.to_string_lossy().into_owned(),
            output_path.to_string_lossy().into_owned(),
        ],
    );

    assert!(
        result.is_ok(),
        "expected block File I/O support; current Duke failed with {result:?}"
    );
    assert_eq!(
        result.unwrap(),
        Some(Slot::Int(4)),
        "copyWithBuffer should report the number of buffered bytes"
    );
    assert_eq!(
        std::fs::read(&output_path).expect("read output file"),
        input_bytes,
        "block copy output bytes should match the input bytes"
    );
}

#[test]
fn file_io_missing_input_raises_file_not_found() {
    let (root, _cleanup) = make_temp_root("duke-file-io-missing");
    let missing_path = root.join("missing.bin");

    let result = run_bootstrap_with_string_args(
        "FileIoTest.class",
        "missingFile",
        "(Ljava/lang/String;)I",
        &[missing_path.to_string_lossy().into_owned()],
    );

    assert_eq!(
        result.unwrap(),
        Some(Slot::Int(1)),
        "missingFile should catch FileNotFoundException"
    );
}

#[test]
fn file_io_read_after_close_raises_io_exception() {
    let (root, _cleanup) = make_temp_root("duke-file-io-read-close");
    let input_path = root.join("input.bin");
    std::fs::write(&input_path, [1_u8, 2, 3]).expect("write input file");

    let result = run_bootstrap_with_string_args(
        "FileIoTest.class",
        "readAfterClose",
        "(Ljava/lang/String;)I",
        &[input_path.to_string_lossy().into_owned()],
    );

    assert_eq!(
        result.unwrap(),
        Some(Slot::Int(1)),
        "readAfterClose should catch IOException"
    );
}

#[test]
fn file_io_write_after_close_raises_io_exception() {
    let (root, _cleanup) = make_temp_root("duke-file-io-write-close");
    let output_path = root.join("output.bin");

    let result = run_bootstrap_with_string_args(
        "FileIoTest.class",
        "writeAfterClose",
        "(Ljava/lang/String;)I",
        &[output_path.to_string_lossy().into_owned()],
    );

    assert_eq!(
        result.unwrap(),
        Some(Slot::Int(1)),
        "writeAfterClose should catch IOException"
    );
}

// ---- Phase 28: Threading ----

#[test]
fn threading_havoc_fast_fail_slow_thread_does_not_panic() {
    static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let ctx = load_class_context("ThreadingTest.class");
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();

    COUNTER.store(0, std::sync::atomic::Ordering::SeqCst);

    registry
        .natives_mut()
        .register("java/lang/Thread", "sleep", "(J)V", |_, _, _, _| {
            let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if count == 0 {
                // First thread to call sleep fails immediately
                Err(VmError::Unimplemented {
                    mnemonic: "Test early failure",
                })
            } else {
                // Other threads take a long time to sleep, so they will be still running
                // if wait_for_all_java_threads returns early.
                std::thread::sleep(std::time::Duration::from_millis(100));
                Ok(None)
            }
        });

    let result = execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut out,
        &entry_class,
        "spawnAndJoinTen",
        "()I",
        &[],
    );

    assert!(matches!(
        result,
        Err(VmError::Unimplemented {
            mnemonic: "Test early failure"
        })
    ));
}

#[test]
fn threading_havoc_wait_for_all_java_threads_error_path() {
    let ctx = load_class_context("ThreadingTest.class");
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();

    registry
        .natives_mut()
        .register("java/lang/Thread", "sleep", "(J)V", |_, _, _, _| {
            Err(VmError::Unimplemented {
                mnemonic: "Test panic simulation",
            })
        });

    let result = execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut out,
        &entry_class,
        "spawnAndJoinTen",
        "()I",
        &[],
    );

    assert!(matches!(
        result,
        Err(VmError::Unimplemented {
            mnemonic: "Test panic simulation"
        })
    ));
}

#[test]
fn threading_havoc_wait_for_all_java_threads_rust_panic_path() {
    let ctx = load_class_context("ThreadingTest.class");
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();

    registry
        .natives_mut()
        .register("java/lang/Thread", "sleep", "(J)V", |_, _, _, _| {
            panic!("Test rust panic simulation");
        });

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = execute_class_to_completion(
            &mut registry,
            loader,
            &mut heap,
            &mut out,
            &entry_class,
            "spawnAndJoinTen",
            "()I",
            &[],
        );
    }));

    assert!(result.is_err(), "Expected the panic to propagate");
    // We don't care about the specific error message as long as it propagated
    // It could be 'Test rust panic simulation' or 'PoisonError'
}
#[test]
fn threading_spawn_and_join_ten_workers() {
    let start = std::time::Instant::now();
    let result = run_bootstrap_with_output("ThreadingTest.class", "spawnAndJoinTen", "()I");
    let elapsed = start.elapsed();

    assert!(
        result.is_ok(),
        "expected basic Thread/Runnable support; current Duke failed with {result:?}"
    );
    let (value, mut lines) = result.unwrap();
    lines.sort();
    assert_eq!(
        value,
        Some(Slot::Int(10)),
        "spawnAndJoinTen should return 10"
    );
    assert_eq!(lines.len(), 20, "ten workers should print two lines each");
    assert!(
        elapsed >= std::time::Duration::from_millis(40),
        "spawnAndJoinTen should observe real sleep time; elapsed={elapsed:?}"
    );
    assert!(
        elapsed < std::time::Duration::from_millis(450),
        "spawnAndJoinTen should complete faster than serialized sleeps; elapsed={elapsed:?}"
    );
    for expected in 0..10 {
        assert!(
            lines.iter().any(|line| line == &expected.to_string()),
            "missing worker start line for {expected}: {lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|line| line == &(expected + 100).to_string()),
            "missing worker finish line for {}: {lines:?}",
            expected + 100
        );
        assert!(
            !lines
                .iter()
                .any(|line| line == &(-1000 - expected).to_string()),
            "worker {expected} reported a sleep failure: {lines:?}"
        );
    }
}

#[test]
fn threading_subclass_run_method_wins_over_base_thread() {
    let result = run_bootstrap_with_output("ThreadingTest.class", "subclassRunWins", "()I");

    assert!(
        result.is_ok(),
        "expected Thread subclass support; current Duke failed with {result:?}"
    );
    let (value, lines) = result.unwrap();
    assert_eq!(value, Some(Slot::Int(1)), "subclassRunWins should return 1");
    assert!(
        lines.iter().any(|line| line == "205"),
        "expected subclass run() output to contain 205, got {lines:?}"
    );
}

#[test]
fn threading_fire_and_forget_still_waits_for_workers_before_returning() {
    let result =
        run_bootstrap_with_output("ThreadingTest.class", "fireAndForgetStillFinishes", "()I");

    assert!(
        result.is_ok(),
        "expected Duke to keep the VM alive for worker threads; current Duke failed with {result:?}"
    );
    let (value, mut lines) = result.unwrap();
    lines.sort();
    assert_eq!(
        value,
        Some(Slot::Int(3)),
        "fireAndForgetStillFinishes should return 3"
    );
    assert_eq!(
        lines.len(),
        6,
        "three workers should still finish before the VM returns"
    );
    for expected in [0, 1, 2, 100, 101, 102] {
        assert!(
            lines.iter().any(|line| line == &expected.to_string()),
            "missing expected worker output {expected}: {lines:?}"
        );
    }
}

#[test]
fn concurrency_two_busy_workers_interleave() {
    // Run the test several times — scheduling is non-deterministic, but
    // with the fair-yield mechanism at least one attempt out of a handful
    // should show interleaving even on single-core CI runners.
    let mut any_interleaved = false;
    let mut last_lines: Vec<String> = Vec::new();

    for _ in 0..5 {
        let result =
            run_bootstrap_with_output("ConcurrencyTest.class", "twoWorkersBusyLoop", "()I");

        assert!(
            result.is_ok(),
            "expected busy-loop concurrency test to pass; failed with {result:?}"
        );
        let (value, lines) = result.unwrap();
        assert_eq!(
            value,
            Some(Slot::Int(2)),
            "twoWorkersBusyLoop should return 2"
        );

        // Each worker prints 4 checkpoint lines.
        assert_eq!(
            lines.len(),
            8,
            "two workers with 4 checkpoints each = 8 lines; got {lines:?}"
        );

        // Verify all expected lines are present.
        for worker in 0..2 {
            for checkpoint in 1..=4 {
                let expected = format!("{worker}:{checkpoint}");
                assert!(
                    lines.iter().any(|l| l == &expected),
                    "missing output line {expected}: {lines:?}"
                );
            }
        }

        // Check interleaving: the output should NOT be perfectly
        // serialised (all of worker 0 before all of worker 1).
        let first_w1 = lines.iter().position(|l| l.starts_with("1:"));
        let last_w0 = lines.iter().rposition(|l| l.starts_with("0:"));
        if let (Some(f1), Some(l0)) = (first_w1, last_w0)
            && f1 < l0
        {
            any_interleaved = true;
            break;
        }
        last_lines = lines;
    }

    assert!(
        any_interleaved,
        "expected interleaved execution in at least one attempt; last output: {last_lines:?}"
    );
}

// ---------------------------------------------------------------------------
// ClassRegistry unit tests
// ---------------------------------------------------------------------------

#[test]
fn class_registry_register_lambda_increments_counter() {
    let mut reg = ClassRegistry::new();
    let info = LambdaInfo {
        impl_class: "Foo".to_string(),
        impl_method: "lambda$0".to_string(),
        impl_desc: "()V".to_string(),
        impl_kind: 6,
        sam_method: "run".to_string(),
        sam_desc: "()V".to_string(),
        captured_count: 0,
    };
    let n0 = reg.register_lambda(info.clone());
    let n1 = reg.register_lambda(info);
    assert_ne!(n0, n1, "each lambda gets a distinct name");
    assert!(n0.contains('0'), "first lambda name contains '0'");
    assert!(n1.contains('1'), "second lambda name contains '1'");
}

#[test]
fn class_registry_natives_returns_registry() {
    #[allow(clippy::unnecessary_wraps)]
    fn dummy(
        _args: &[Slot],
        _heap: &mut duke_gc::Heap,
        _out: &mut dyn std::io::Write,
        _control: &mut NativeControl,
    ) -> VmResult<Option<Slot>> {
        Ok(Some(Slot::Int(99)))
    }
    let mut reg = ClassRegistry::new();
    reg.natives_mut().register("C", "m", "()I", dummy);
    assert!(
        reg.natives_mut().get_kind("C", "m", "()I").is_some(),
        "natives() getter must expose registered handler"
    );
}

#[test]
fn class_registry_contains_true_after_register() {
    let mut reg = ClassRegistry::new();
    let ctx = ClassContext {
        class_name: "Foo".to_string(),
        super_class: None,
        interfaces: vec![],
        constant_pool: vec![],
        methods: vec![],
        fields: vec![],
        static_fields: vec![],
        instance_field_count: 0,
        bootstrap_methods: vec![],
    };
    assert!(!reg.contains("Foo"));
    reg.register(ctx);
    assert!(reg.contains("Foo"));
}

#[test]
fn class_registry_all_classes_counts_after_bootstrap() {
    let mut reg = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut reg, &mut heap);
    let count = reg.all_classes().count();
    assert!(count > 10, "bootstrap registers many classes; got {count}");
}

#[test]
fn class_registry_all_classes_mut_allows_mutation() {
    let mut reg = ClassRegistry::new();
    let ctx = ClassContext {
        class_name: "Bar".to_string(),
        super_class: None,
        interfaces: vec![],
        constant_pool: vec![],
        methods: vec![],
        fields: vec![],
        static_fields: vec![Slot::Int(1)],
        instance_field_count: 0,
        bootstrap_methods: vec![],
    };
    reg.register(ctx);
    for cls in reg.all_classes_mut() {
        for slot in &mut cls.static_fields {
            if let Slot::Int(v) = slot {
                *v = 42;
            }
        }
    }
    let bar = reg.get("Bar").unwrap();
    assert_eq!(bar.static_fields[0], Slot::Int(42));
}

// ---------------------------------------------------------------------------
// heap_object_to_string
// ---------------------------------------------------------------------------

#[test]
fn heap_object_to_string_integer_field() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(42);
    let obj = heap.get(r).unwrap().clone();
    assert_eq!(heap_object_to_string(&obj, r), "42");
}

#[test]
fn heap_object_to_string_long_field() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Long(999_000_000_000_i64);
    let obj = heap.get(r).unwrap().clone();
    assert_eq!(heap_object_to_string(&obj, r), "999000000000");
}

#[test]
fn heap_object_to_string_double_field() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Double".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Double(std::f64::consts::PI);
    let obj = heap.get(r).unwrap().clone();
    let s = heap_object_to_string(&obj, r);
    assert!(s.contains("3.14"), "expected '3.14' in '{s}'");
}

#[test]
fn heap_object_to_string_float_field() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Float".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Float(1.5_f32);
    let obj = heap.get(r).unwrap().clone();
    assert_eq!(heap_object_to_string(&obj, r), "1.5");
}

#[test]
fn heap_object_to_string_boolean_true() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Boolean".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(1);
    let obj = heap.get(r).unwrap().clone();
    assert_eq!(heap_object_to_string(&obj, r), "true");
}

#[test]
fn heap_object_to_string_boolean_false() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Boolean".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(0);
    let obj = heap.get(r).unwrap().clone();
    assert_eq!(heap_object_to_string(&obj, r), "false");
}

#[test]
fn heap_object_to_string_boolean_nonzero_is_true() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Boolean".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(7);
    let obj = heap.get(r).unwrap().clone();
    assert_eq!(heap_object_to_string(&obj, r), "true");
}

#[test]
fn heap_object_to_string_character_field() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Character".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int('A' as i32);
    let obj = heap.get(r).unwrap().clone();
    assert_eq!(heap_object_to_string(&obj, r), "A");
}

#[test]
fn heap_object_to_string_string_value_takes_priority() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate_string("hello".to_string());
    let obj = heap.get(r).unwrap().clone();
    assert_eq!(heap_object_to_string(&obj, r), "hello");
}

#[test]
fn heap_object_to_string_opaque_object_uses_class_at_hex() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Object".to_string(), 0);
    let obj = heap.get(r).unwrap().clone();
    let s = heap_object_to_string(&obj, r);
    assert!(
        s.starts_with("java/lang/Object@"),
        "expected 'java/lang/Object@...' but got '{s}'"
    );
}

// ---------------------------------------------------------------------------
// format_java_float / format_java_double
// ---------------------------------------------------------------------------

#[test]
fn format_java_float_nan() {
    assert_eq!(format_java_float(f32::NAN), "NaN");
}

#[test]
fn format_java_float_positive_infinity() {
    assert_eq!(format_java_float(f32::INFINITY), "Infinity");
}

#[test]
fn format_java_float_negative_infinity() {
    assert_eq!(format_java_float(f32::NEG_INFINITY), "-Infinity");
}

#[test]
fn format_java_float_finite_with_decimal() {
    let s = format_java_float(std::f32::consts::PI);
    assert!(s.contains('.'), "finite float must contain '.': {s}");
}

#[test]
fn format_java_float_whole_number_gets_dot_zero() {
    let s = format_java_float(2.0_f32);
    assert!(
        s.ends_with(".0") || s.contains('.'),
        "must have decimal: {s}"
    );
}

#[test]
fn format_java_double_nan() {
    assert_eq!(format_java_double(f64::NAN), "NaN");
}

#[test]
fn format_java_double_positive_infinity() {
    assert_eq!(format_java_double(f64::INFINITY), "Infinity");
}

#[test]
fn format_java_double_negative_infinity() {
    assert_eq!(format_java_double(f64::NEG_INFINITY), "-Infinity");
}

#[test]
fn format_java_double_finite_with_decimal() {
    let s = format_java_double(std::f64::consts::E);
    assert!(s.contains('.'), "finite double must contain '.': {s}");
}

// ---------------------------------------------------------------------------
// native_println_string / native_print_string null arms
// ---------------------------------------------------------------------------

#[test]
fn native_println_string_null_ref_prints_null() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let result = native_println_string(
        &[Slot::Reference(None), Slot::Reference(None)],
        &mut heap,
        &mut out,
    );
    assert!(result.is_ok());
    assert_eq!(String::from_utf8(out).unwrap().trim(), "null");
}

#[test]
fn native_print_string_null_ref_prints_null() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let result = native_print_string(
        &[Slot::Reference(None), Slot::Reference(None)],
        &mut heap,
        &mut out,
    );
    assert!(result.is_ok());
    assert_eq!(String::from_utf8(out).unwrap(), "null");
}

// ---------------------------------------------------------------------------
// native_println_boolean / native_print_boolean
// ---------------------------------------------------------------------------

#[test]
fn native_println_boolean_false() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    native_println_boolean(&[Slot::Reference(None), Slot::Int(0)], &mut heap, &mut out)
        .unwrap();
    assert_eq!(String::from_utf8(out).unwrap().trim(), "false");
}

#[test]
fn native_println_boolean_true() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    native_println_boolean(&[Slot::Reference(None), Slot::Int(1)], &mut heap, &mut out)
        .unwrap();
    assert_eq!(String::from_utf8(out).unwrap().trim(), "true");
}

#[test]
fn native_print_boolean_false() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    native_print_boolean(&[Slot::Reference(None), Slot::Int(0)], &mut heap, &mut out).unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), "false");
}

#[test]
fn native_print_boolean_true() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    native_print_boolean(&[Slot::Reference(None), Slot::Int(5)], &mut heap, &mut out).unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), "true");
}

// ---------------------------------------------------------------------------
// native_print_char / native_print_long / native_print_float / native_print_double
// ---------------------------------------------------------------------------

#[test]
fn native_print_char_ascii() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    native_print_char(
        &[Slot::Reference(None), Slot::Int('Z' as i32)],
        &mut heap,
        &mut out,
    )
    .unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), "Z");
}

#[test]
fn native_print_long_value() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    native_print_long(
        &[Slot::Reference(None), Slot::Long(123_456_789_000_i64)],
        &mut heap,
        &mut out,
    )
    .unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), "123456789000");
}

#[test]
fn native_print_float_value() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    native_print_float(
        &[Slot::Reference(None), Slot::Float(3.0_f32)],
        &mut heap,
        &mut out,
    )
    .unwrap();
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains('3'), "expected '3' in '{s}'");
}

#[test]
fn native_print_int_value() {
    let mut heap = duke_gc::Heap::new();
    let mut out = Vec::new();
    let mut control = NativeControl::default();
    native_print_int(
        &[Slot::Reference(None), Slot::Int(42)],
        &mut heap,
        &mut out,
        &mut control,
    )
    .unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), "42");
}

#[test]
fn native_print_int_type_mismatch() {
    let mut heap = duke_gc::Heap::new();
    let mut out = Vec::new();
    let mut control = NativeControl::default();
    let err = native_print_int(
        &[Slot::Reference(None), Slot::Long(42)],
        &mut heap,
        &mut out,
        &mut control,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::TypeMismatch { .. }));
}

#[test]
fn native_println_int_value() {
    let mut heap = duke_gc::Heap::new();
    let mut out = Vec::new();
    let mut control = NativeControl::default();
    native_println_int(
        &[Slot::Reference(None), Slot::Int(100)],
        &mut heap,
        &mut out,
        &mut control,
    )
    .unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), "100\n");
}

#[test]
fn native_println_int_type_mismatch() {
    let mut heap = duke_gc::Heap::new();
    let mut out = Vec::new();
    let mut control = NativeControl::default();
    let err = native_println_int(
        &[Slot::Reference(None), Slot::Float(100.0)],
        &mut heap,
        &mut out,
        &mut control,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::TypeMismatch { .. }));
}

#[test]
fn native_print_double_value() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    native_print_double(
        &[Slot::Reference(None), Slot::Double(2.5)],
        &mut heap,
        &mut out,
    )
    .unwrap();
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains("2.5"), "expected '2.5' in '{s}'");
}

// ---------------------------------------------------------------------------
// native_println_object / native_print_object
// ---------------------------------------------------------------------------

#[test]
fn native_println_object_null_prints_null() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    native_println_object(
        &[Slot::Reference(None), Slot::Reference(None)],
        &mut heap,
        &mut out,
    )
    .unwrap();
    assert_eq!(String::from_utf8(out).unwrap().trim(), "null");
}

#[test]
fn native_println_object_nonnull_uses_string_value() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate_string("hi".to_string());
    let mut out: Vec<u8> = Vec::new();
    native_println_object(
        &[Slot::Reference(None), Slot::Reference(Some(r))],
        &mut heap,
        &mut out,
    )
    .unwrap();
    assert_eq!(String::from_utf8(out).unwrap().trim(), "hi");
}

#[test]
fn native_print_object_null_prints_null() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    native_print_object(
        &[Slot::Reference(None), Slot::Reference(None)],
        &mut heap,
        &mut out,
    )
    .unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), "null");
}

#[test]
fn native_print_object_nonnull() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate_string("world".to_string());
    let mut out: Vec<u8> = Vec::new();
    native_print_object(
        &[Slot::Reference(None), Slot::Reference(Some(r))],
        &mut heap,
        &mut out,
    )
    .unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), "world");
}

// ---------------------------------------------------------------------------
// native_object_tostring
// ---------------------------------------------------------------------------

#[test]
fn native_object_tostring_returns_string_ref() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate_string("test".to_string());
    let mut out: Vec<u8> = Vec::new();
    let result = native_object_tostring(&[Slot::Reference(Some(r))], &mut heap, &mut out)
        .unwrap()
        .unwrap();
    let new_ref = result.as_reference().unwrap();
    let s = heap.get(new_ref).unwrap().string_value.as_deref().unwrap();
    assert_eq!(s, "test");
}

#[test]
fn native_object_equals_uses_reference_identity() {
    let mut heap = duke_gc::Heap::new();
    let a = heap.allocate("java/lang/Object".to_string(), 0);
    let b = heap.allocate("java/lang/Object".to_string(), 0);
    let mut out: Vec<u8> = Vec::new();

    let same = native_object_equals(
        &[Slot::Reference(Some(a)), Slot::Reference(Some(a))],
        &mut heap,
        &mut out,
        &mut NativeControl::default(),
    )
    .unwrap();
    let different = native_object_equals(
        &[Slot::Reference(Some(a)), Slot::Reference(Some(b))],
        &mut heap,
        &mut out,
        &mut NativeControl::default(),
    )
    .unwrap();
    let null_other = native_object_equals(
        &[Slot::Reference(Some(a)), Slot::Reference(None)],
        &mut heap,
        &mut out,
        &mut NativeControl::default(),
    )
    .unwrap();

    assert_eq!(same, Some(Slot::Int(1)));
    assert_eq!(different, Some(Slot::Int(0)));
    assert_eq!(null_other, Some(Slot::Int(0)));
}

// ---------------------------------------------------------------------------
// native_string_equals null arm
// ---------------------------------------------------------------------------

#[test]
fn native_string_equals_null_other_returns_false() {
    let mut heap = duke_gc::Heap::new();
    let this = heap.allocate_string("hello".to_string());
    let mut out: Vec<u8> = Vec::new();
    let result = native_string_equals(
        &[Slot::Reference(Some(this)), Slot::Reference(None)],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    assert_eq!(result, Slot::Int(0));
}

// ---------------------------------------------------------------------------
// native_system_exit
// ---------------------------------------------------------------------------

#[test]
fn native_system_exit_returns_system_exit_error() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let err = native_system_exit(&[Slot::Int(42)], &mut heap, &mut out).unwrap_err();
    assert!(
        matches!(err, VmError::SystemExit { code: 42 }),
        "expected SystemExit(42), got {err:?}"
    );
}

#[test]
fn native_system_exit_non_int_arg_uses_code_1() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let err = native_system_exit(&[], &mut heap, &mut out).unwrap_err();
    assert!(
        matches!(err, VmError::SystemExit { code: 1 }),
        "empty args → SystemExit(1), got {err:?}"
    );
}

#[test]
fn native_system_get_property_returns_temp_dir_and_null_for_unknown() {
    let mut heap = duke_gc::Heap::new();
    let key_ref = heap.allocate_string("java.io.tmpdir".to_string());
    let unknown_ref = heap.allocate_string("duke.missing.property".to_string());
    let mut out: Vec<u8> = Vec::new();

    let temp_result = native_system_get_property(
        &[Slot::Reference(Some(key_ref))],
        &mut heap,
        &mut out,
        &mut NativeControl::default(),
    )
    .unwrap();
    let unknown_result = native_system_get_property(
        &[Slot::Reference(Some(unknown_ref))],
        &mut heap,
        &mut out,
        &mut NativeControl::default(),
    )
    .unwrap();

    let Slot::Reference(Some(temp_ref)) = temp_result.unwrap() else {
        panic!("expected temp dir string reference");
    };
    assert_eq!(
        heap.get(temp_ref).unwrap().string_value.as_deref(),
        Some(std::env::temp_dir().to_string_lossy().as_ref())
    );
    assert_eq!(unknown_result, Some(Slot::Reference(None)));
}

#[test]
fn native_system_set_property_overrides_get_property() {
    let mut heap = duke_gc::Heap::new();
    let key_ref = heap.allocate_string("duke.test.set.property".to_string());
    let value_ref = heap.allocate_string("boot-loader".to_string());
    let mut out: Vec<u8> = Vec::new();

    let previous = native_system_set_property(
        &[
            Slot::Reference(Some(key_ref)),
            Slot::Reference(Some(value_ref)),
        ],
        &mut heap,
        &mut out,
        &mut NativeControl::default(),
    )
    .unwrap();
    let current = native_system_get_property(
        &[Slot::Reference(Some(key_ref))],
        &mut heap,
        &mut out,
        &mut NativeControl::default(),
    )
    .unwrap();

    assert_eq!(previous, Some(Slot::Reference(None)));
    let Some(Slot::Reference(Some(current_ref))) = current else {
        panic!("expected property value string");
    };
    assert_eq!(
        string_value_from_ref(&heap, current_ref).unwrap(),
        "boot-loader"
    );
}

// ---------------------------------------------------------------------------
// native_string_substring boundary / error
// ---------------------------------------------------------------------------

#[test]
fn native_string_substring_full_string() {
    let mut heap = duke_gc::Heap::new();
    let this = heap.allocate_string("hello".to_string());
    let mut out: Vec<u8> = Vec::new();
    let r = native_string_substring(
        &[Slot::Reference(Some(this)), Slot::Int(0)],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    let sub_ref = r.as_reference().unwrap();
    assert_eq!(
        heap.get(sub_ref).unwrap().string_value.as_deref(),
        Some("hello")
    );
}

#[test]
fn native_string_substring_tail() {
    let mut heap = duke_gc::Heap::new();
    let this = heap.allocate_string("hello".to_string());
    let mut out: Vec<u8> = Vec::new();
    let r = native_string_substring(
        &[Slot::Reference(Some(this)), Slot::Int(2)],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    let sub_ref = r.as_reference().unwrap();
    assert_eq!(
        heap.get(sub_ref).unwrap().string_value.as_deref(),
        Some("llo")
    );
}

#[test]
fn native_string_substring_out_of_bounds_returns_error() {
    let mut heap = duke_gc::Heap::new();
    let this = heap.allocate_string("hi".to_string());
    let mut out: Vec<u8> = Vec::new();
    let err = native_string_substring(
        &[Slot::Reference(Some(this)), Slot::Int(10)],
        &mut heap,
        &mut out,
    )
    .unwrap_err();
    assert!(
        matches!(err, VmError::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
}

#[test]
fn native_string_substring_range_basic() {
    let mut heap = duke_gc::Heap::new();
    let this = heap.allocate_string("hello".to_string());
    let mut out: Vec<u8> = Vec::new();
    let r = native_string_substring_range(
        &[Slot::Reference(Some(this)), Slot::Int(1), Slot::Int(4)],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    let sub_ref = r.as_reference().unwrap();
    assert_eq!(
        heap.get(sub_ref).unwrap().string_value.as_deref(),
        Some("ell")
    );
}

#[test]
fn native_string_substring_range_out_of_bounds() {
    let mut heap = duke_gc::Heap::new();
    let this = heap.allocate_string("hi".to_string());
    let mut out: Vec<u8> = Vec::new();
    let err = native_string_substring_range(
        &[Slot::Reference(Some(this)), Slot::Int(0), Slot::Int(10)],
        &mut heap,
        &mut out,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---------------------------------------------------------------------------
// format_arg
// ---------------------------------------------------------------------------

#[test]
fn format_arg_long_d_spec() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Long(12345_i64);
    let result = format_arg('d', None, &Slot::Reference(Some(r)), &heap).unwrap();
    assert_eq!(result, "12345");
}

#[test]
fn format_arg_double_f_spec() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Double".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Double(std::f64::consts::PI);
    let result = format_arg('f', Some(2), &Slot::Reference(Some(r)), &heap).unwrap();
    assert_eq!(result, "3.14");
}

#[test]
fn format_arg_float_f_spec() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Float".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Float(1.5_f32);
    let result = format_arg('f', Some(1), &Slot::Reference(Some(r)), &heap).unwrap();
    assert_eq!(result, "1.5");
}

#[test]
fn format_arg_int_x_spec() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(255);
    let result = format_arg('x', None, &Slot::Reference(Some(r)), &heap).unwrap();
    assert_eq!(result, "ff");
}

#[test]
fn format_arg_int_uppercase_x_spec() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(255);
    let result = format_arg('X', None, &Slot::Reference(Some(r)), &heap).unwrap();
    assert_eq!(result, "FF");
}

#[test]
fn format_arg_long_x_spec() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Long(255_i64);
    let result = format_arg('x', None, &Slot::Reference(Some(r)), &heap).unwrap();
    assert_eq!(result, "ff");
}

#[test]
fn format_arg_null_returns_null_string() {
    let heap = duke_gc::Heap::new();
    let result = format_arg('s', None, &Slot::Reference(None), &heap).unwrap();
    assert_eq!(result, "null");
}

// ---------------------------------------------------------------------------
// native_string_format
// ---------------------------------------------------------------------------

#[test]
fn native_string_format_percent_d() {
    let mut heap = duke_gc::Heap::new();
    let fmt_ref = heap.allocate_string("%d".to_string());
    let arr_ref = heap.allocate("[Ljava/lang/Object;".to_string(), 1);
    let int_obj = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(int_obj).unwrap().fields[0] = Slot::Int(7);
    heap.get_mut(arr_ref).unwrap().fields[0] = Slot::Reference(Some(int_obj));
    let mut out: Vec<u8> = Vec::new();
    let result = native_string_format(
        &[
            Slot::Reference(Some(fmt_ref)),
            Slot::Reference(Some(arr_ref)),
        ],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    let s_ref = result.as_reference().unwrap();
    assert_eq!(heap.get(s_ref).unwrap().string_value.as_deref(), Some("7"));
}

#[test]
fn native_string_format_percent_n() {
    let mut heap = duke_gc::Heap::new();
    let fmt_ref = heap.allocate_string("a%nb".to_string());
    let arr_ref = heap.allocate("[Ljava/lang/Object;".to_string(), 0);
    let mut out: Vec<u8> = Vec::new();
    let result = native_string_format(
        &[
            Slot::Reference(Some(fmt_ref)),
            Slot::Reference(Some(arr_ref)),
        ],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    let s_ref = result.as_reference().unwrap();
    assert_eq!(
        heap.get(s_ref).unwrap().string_value.as_deref(),
        Some("a\nb")
    );
}

#[test]
fn native_string_format_percent_percent() {
    let mut heap = duke_gc::Heap::new();
    let fmt_ref = heap.allocate_string("100%%".to_string());
    let arr_ref = heap.allocate("[Ljava/lang/Object;".to_string(), 0);
    let mut out: Vec<u8> = Vec::new();
    let result = native_string_format(
        &[
            Slot::Reference(Some(fmt_ref)),
            Slot::Reference(Some(arr_ref)),
        ],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    let s_ref = result.as_reference().unwrap();
    assert_eq!(
        heap.get(s_ref).unwrap().string_value.as_deref(),
        Some("100%")
    );
}

#[test]
fn native_string_format_precision() {
    let mut heap = duke_gc::Heap::new();
    let fmt_ref = heap.allocate_string("%.2f".to_string());
    let arr_ref = heap.allocate("[Ljava/lang/Object;".to_string(), 1);
    let dbl_obj = heap.allocate("java/lang/Double".to_string(), 1);
    heap.get_mut(dbl_obj).unwrap().fields[0] = Slot::Double(std::f64::consts::PI);
    heap.get_mut(arr_ref).unwrap().fields[0] = Slot::Reference(Some(dbl_obj));
    let mut out: Vec<u8> = Vec::new();
    let result = native_string_format(
        &[
            Slot::Reference(Some(fmt_ref)),
            Slot::Reference(Some(arr_ref)),
        ],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    let s_ref = result.as_reference().unwrap();
    assert_eq!(
        heap.get(s_ref).unwrap().string_value.as_deref(),
        Some("3.14")
    );
}

// ---------------------------------------------------------------------------
// execute_string_concat_recipe
// ---------------------------------------------------------------------------

#[test]
fn execute_string_concat_recipe_single_dynamic_int() {
    let mut heap = duke_gc::Heap::new();
    let slot = execute_string_concat_recipe("\u{1}", &[Slot::Int(42)], &['I'], &[], &mut heap)
        .unwrap();
    let r = slot.as_reference().unwrap();
    assert_eq!(heap.get(r).unwrap().string_value.as_deref(), Some("42"));
}

#[test]
fn execute_string_concat_recipe_constant_only() {
    let mut heap = duke_gc::Heap::new();
    let slot =
        execute_string_concat_recipe("\u{2}", &[], &[], &["hello".to_string()], &mut heap)
            .unwrap();
    let r = slot.as_reference().unwrap();
    assert_eq!(heap.get(r).unwrap().string_value.as_deref(), Some("hello"));
}

#[test]
fn execute_string_concat_recipe_literal_chars() {
    let mut heap = duke_gc::Heap::new();
    let slot = execute_string_concat_recipe("xyz", &[], &[], &[], &mut heap).unwrap();
    let r = slot.as_reference().unwrap();
    assert_eq!(heap.get(r).unwrap().string_value.as_deref(), Some("xyz"));
}

#[test]
fn execute_string_concat_recipe_mixed() {
    let mut heap = duke_gc::Heap::new();
    let slot = execute_string_concat_recipe("x\u{1}y", &[Slot::Int(5)], &['I'], &[], &mut heap)
        .unwrap();
    let r = slot.as_reference().unwrap();
    assert_eq!(heap.get(r).unwrap().string_value.as_deref(), Some("x5y"));
}

#[test]
fn execute_string_concat_recipe_multiple_dynamics() {
    let mut heap = duke_gc::Heap::new();
    let slot = execute_string_concat_recipe(
        "\u{1}+\u{1}",
        &[Slot::Int(3), Slot::Int(4)],
        &['I', 'I'],
        &[],
        &mut heap,
    )
    .unwrap();
    let r = slot.as_reference().unwrap();
    assert_eq!(heap.get(r).unwrap().string_value.as_deref(), Some("3+4"));
}

// ---------------------------------------------------------------------------
// Ishr / Iushr masking via execute()
// ---------------------------------------------------------------------------

#[test]
fn execute_ishr_masks_shift_amount() {
    // Arithmetic right shift: -8 >> 1 = -4 (sign-extending)
    // Also verifies s & 0x1F: shift=1 means bit mask = 1
    let instrs = vec![
        (0, Instruction::Bipush(-8_i8)),
        (2, Instruction::Bipush(1_i8)),
        (4, Instruction::Ishr),
        (5, Instruction::Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 4, 1).unwrap();
    assert_eq!(result, Some(Slot::Int(-4)));
}

#[test]
fn execute_ishr_large_shift_masked_to_31() {
    // shift = 33 → 33 & 0x1F = 1; -8 >> 1 = -4
    let instrs = vec![
        (0, Instruction::Bipush(-8_i8)),
        (2, Instruction::Bipush(33_i8)),
        (4, Instruction::Ishr),
        (5, Instruction::Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 4, 1).unwrap();
    assert_eq!(result, Some(Slot::Int(-4)));
}

#[test]
fn execute_iushr_masks_shift_amount() {
    // Logical right shift: -1 (0xFFFFFFFF) >>> 28 = 15
    let instrs = vec![
        (0, Instruction::IconstM1),
        (1, Instruction::Bipush(28_i8)),
        (3, Instruction::Iushr),
        (4, Instruction::Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 4, 1).unwrap();
    assert_eq!(result, Some(Slot::Int(15)));
}

#[test]
fn execute_iushr_large_shift_masked() {
    // shift=60 → 60 & 0x1F = 28; -1 >>> 28 = 15
    let instrs = vec![
        (0, Instruction::IconstM1),
        (1, Instruction::Bipush(60_i8)),
        (3, Instruction::Iushr),
        (4, Instruction::Ireturn),
    ];
    let result = execute(&instrs, &[], vec![], 4, 1).unwrap();
    assert_eq!(result, Some(Slot::Int(15)));
}

// ---------------------------------------------------------------------------
// Lshr / Lushr masking via execute()
// ---------------------------------------------------------------------------

#[test]
fn execute_lshr_arithmetic_shift() {
    // long -8 >> 1 = -4 (arithmetic; sign bit preserved)
    let instrs = vec![
        (0, Instruction::Lload0),
        (1, Instruction::Bipush(1_i8)),
        (3, Instruction::Lshr),
        (4, Instruction::Lreturn),
    ];
    let result = execute(&instrs, &[], vec![Slot::Long(-8_i64)], 4, 2).unwrap();
    assert_eq!(result, Some(Slot::Long(-4)));
}

#[test]
fn execute_lushr_logical_shift() {
    // long -1 (0xFFFFFFFFFFFFFFFF) >>> 60 = 15
    use duke_bytecode::Instruction;
    let instrs = vec![
        (0, Instruction::Lload0),
        (1, Instruction::Bipush(60_i8)),
        (3, Instruction::Lushr),
        (4, Instruction::Lreturn),
    ];
    let result = execute(&instrs, &[], vec![Slot::Long(-1_i64)], 4, 2).unwrap();
    assert_eq!(result, Some(Slot::Long(15)));
}

#[test]
fn execute_lshr_large_shift_masked_to_63() {
    // shift = 65 → 65 & 0x3F = 1; -8 >> 1 = -4
    use duke_bytecode::Instruction;
    let instrs = vec![
        (0, Instruction::Lload0),
        (1, Instruction::Bipush(65_i8)),
        (3, Instruction::Lshr),
        (4, Instruction::Lreturn),
    ];
    let result = execute(&instrs, &[], vec![Slot::Long(-8_i64)], 4, 2).unwrap();
    assert_eq!(result, Some(Slot::Long(-4)));
}

// ---------------------------------------------------------------------------
// LDC CpEntry::Utf8 arm — via execute() with a hand-crafted CP
// ---------------------------------------------------------------------------

#[test]
fn execute_ldc_string_from_utf8_cp() {
    use duke_classfile::types::{CpEntry, CpIndex};
    // CP: [None, Some(String{string_index:2}), Some(Utf8("hi"))]
    let cp: Vec<Option<CpEntry>> = vec![
        None,
        Some(CpEntry::String {
            string_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("hi".to_string())),
    ];
    // Ldc takes u8 cp index; execute() doesn't implement Areturn so just Pop+Iconst0+Ireturn
    let instrs = vec![
        (0, Instruction::Ldc(1u8)),
        (2, Instruction::Pop),
        (3, Instruction::Iconst0),
        (4, Instruction::Ireturn),
    ];
    // Must not error — exercises the CpEntry::Utf8 branch
    let result = execute(&instrs, &cp, vec![], 2, 1).unwrap();
    assert_eq!(result, Some(Slot::Int(0)));
}

#[test]
fn execute_ldcw_string_from_utf8_cp() {
    use duke_classfile::types::{CpEntry, CpIndex};
    let cp: Vec<Option<CpEntry>> = vec![
        None,
        Some(CpEntry::String {
            string_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("world".to_string())),
    ];
    let instrs = vec![
        (0, Instruction::LdcW(CpIndex(1))),
        (3, Instruction::Pop),
        (4, Instruction::Iconst1),
        (5, Instruction::Ireturn),
    ];
    let result = execute(&instrs, &cp, vec![], 2, 1).unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

// ---------------------------------------------------------------------------
// native_hashmap_remove
// ---------------------------------------------------------------------------

#[test]
fn native_hashmap_remove_existing_key_returns_value_and_decrements_size() {
    let mut heap = duke_gc::Heap::new();
    // Build map: fields = [Int(1), key_ref, val_ref]
    let map = heap.allocate("java/util/HashMap".to_string(), 1);
    let key = heap.allocate_string("k".to_string());
    let val = heap.allocate_string("v".to_string());
    {
        let obj = heap.get_mut(map).unwrap();
        obj.fields[0] = Slot::Int(1); // size
        obj.fields.push(Slot::Reference(Some(key)));
        obj.fields.push(Slot::Reference(Some(val)));
    }
    let mut out: Vec<u8> = Vec::new();
    let result = native_hashmap_remove(
        &[Slot::Reference(Some(map)), Slot::Reference(Some(key))],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    assert_eq!(result, Slot::Reference(Some(val)));
    // Size should be 0 now
    assert_eq!(heap.get(map).unwrap().fields[0], Slot::Int(0));
}

#[test]
fn native_hashmap_remove_absent_key_returns_null() {
    let mut heap = duke_gc::Heap::new();
    let map = heap.allocate("java/util/HashMap".to_string(), 1);
    heap.get_mut(map).unwrap().fields[0] = Slot::Int(0);
    let missing_key = heap.allocate_string("missing".to_string());
    let mut out: Vec<u8> = Vec::new();
    let result = native_hashmap_remove(
        &[
            Slot::Reference(Some(map)),
            Slot::Reference(Some(missing_key)),
        ],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    assert_eq!(result, Slot::Reference(None));
}

// ---------------------------------------------------------------------------
// native_hashmap_get_or_default
// ---------------------------------------------------------------------------

#[test]
fn native_hashmap_get_or_default_key_present() {
    let mut heap = duke_gc::Heap::new();
    let map = heap.allocate("java/util/HashMap".to_string(), 1);
    let key = heap.allocate_string("k".to_string());
    let val = heap.allocate_string("v".to_string());
    let def = heap.allocate_string("default".to_string());
    {
        let obj = heap.get_mut(map).unwrap();
        obj.fields[0] = Slot::Int(1);
        obj.fields.push(Slot::Reference(Some(key)));
        obj.fields.push(Slot::Reference(Some(val)));
    }
    let mut out: Vec<u8> = Vec::new();
    let result = native_hashmap_get_or_default(
        &[
            Slot::Reference(Some(map)),
            Slot::Reference(Some(key)),
            Slot::Reference(Some(def)),
        ],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    assert_eq!(result, Slot::Reference(Some(val)));
}

#[test]
fn native_hashmap_get_or_default_key_absent_returns_default() {
    let mut heap = duke_gc::Heap::new();
    let map = heap.allocate("java/util/HashMap".to_string(), 1);
    heap.get_mut(map).unwrap().fields[0] = Slot::Int(0);
    let missing = heap.allocate_string("nope".to_string());
    let def = heap.allocate_string("fallback".to_string());
    let mut out: Vec<u8> = Vec::new();
    let result = native_hashmap_get_or_default(
        &[
            Slot::Reference(Some(map)),
            Slot::Reference(Some(missing)),
            Slot::Reference(Some(def)),
        ],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    assert_eq!(result, Slot::Reference(Some(def)));
}

// ---------------------------------------------------------------------------
// null-arm tests: String.indexOf, String.contains
// ---------------------------------------------------------------------------

#[test]
fn native_string_indexof_null_target_raises_npe() {
    let mut heap = duke_gc::Heap::new();
    let this = heap.allocate_string("hello".to_string());
    let mut out: Vec<u8> = Vec::new();
    let err = native_string_indexof(
        &[Slot::Reference(Some(this)), Slot::Reference(None)],
        &mut heap,
        &mut out,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::NullPointerException));
}

#[test]
fn native_string_contains_null_target_raises_npe() {
    let mut heap = duke_gc::Heap::new();
    let this = heap.allocate_string("hello".to_string());
    let mut out: Vec<u8> = Vec::new();
    let err = native_string_contains(
        &[Slot::Reference(Some(this)), Slot::Reference(None)],
        &mut heap,
        &mut out,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::NullPointerException));
}

// ---------------------------------------------------------------------------
// null-arm: Integer.parseInt, Long.parseLong, Double.parseDouble,
//           Float.parseFloat, Boolean.parseBoolean
// ---------------------------------------------------------------------------

#[test]
fn native_integer_parseint_null_raises_npe() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let err =
        native_integer_parseint(&[Slot::Reference(None)], &mut heap, &mut out).unwrap_err();
    assert!(matches!(err, VmError::NullPointerException));
}

#[test]
fn native_integer_parseint_valid_string() {
    let mut heap = duke_gc::Heap::new();
    let s = heap.allocate_string("42".to_string());
    let mut out: Vec<u8> = Vec::new();
    let r = native_integer_parseint(&[Slot::Reference(Some(s))], &mut heap, &mut out)
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(42));
}

#[test]
fn native_long_parselong_null_raises_npe() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let err = native_long_parselong(&[Slot::Reference(None)], &mut heap, &mut out).unwrap_err();
    assert!(matches!(err, VmError::NullPointerException));
}

#[test]
fn native_long_parselong_valid_string() {
    let mut heap = duke_gc::Heap::new();
    let s = heap.allocate_string("99999999999".to_string());
    let mut out: Vec<u8> = Vec::new();
    let r = native_long_parselong(&[Slot::Reference(Some(s))], &mut heap, &mut out)
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Long(99_999_999_999_i64));
}

#[test]
fn native_double_parsedouble_null_raises_npe() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let err =
        native_double_parsedouble(&[Slot::Reference(None)], &mut heap, &mut out).unwrap_err();
    assert!(matches!(err, VmError::NullPointerException));
}

#[test]
fn native_double_parsedouble_valid_string() {
    let mut heap = duke_gc::Heap::new();
    let s = heap.allocate_string("2.5".to_string());
    let mut out: Vec<u8> = Vec::new();
    let r = native_double_parsedouble(&[Slot::Reference(Some(s))], &mut heap, &mut out)
        .unwrap()
        .unwrap();
    assert!(matches!(r, Slot::Double(v) if (v - 2.5).abs() < 1e-9));
}

#[test]
fn native_double_doublevalue_nonnull() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Double".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Double(2.5);
    let mut out: Vec<u8> = Vec::new();
    let result = native_double_doublevalue(&[Slot::Reference(Some(r))], &mut heap, &mut out)
        .unwrap()
        .unwrap();
    assert!(matches!(result, Slot::Double(v) if (v - 2.5).abs() < 1e-9));
}

#[test]
fn native_float_parsefloat_null_raises_npe() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let err =
        native_float_parsefloat(&[Slot::Reference(None)], &mut heap, &mut out).unwrap_err();
    assert!(matches!(err, VmError::NullPointerException));
}

#[test]
fn native_float_parsefloat_valid_string() {
    let mut heap = duke_gc::Heap::new();
    let s = heap.allocate_string("1.5".to_string());
    let mut out: Vec<u8> = Vec::new();
    let r = native_float_parsefloat(&[Slot::Reference(Some(s))], &mut heap, &mut out)
        .unwrap()
        .unwrap();
    assert!(matches!(r, Slot::Float(v) if (v - 1.5_f32).abs() < 1e-6));
}

#[test]
fn native_boolean_parseboolean_null_returns_false() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let r = native_boolean_parseboolean(&[Slot::Reference(None)], &mut heap, &mut out)
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn native_boolean_parseboolean_true_string() {
    let mut heap = duke_gc::Heap::new();
    let s = heap.allocate_string("true".to_string());
    let mut out: Vec<u8> = Vec::new();
    let r = native_boolean_parseboolean(&[Slot::Reference(Some(s))], &mut heap, &mut out)
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn native_boolean_parseboolean_false_string() {
    let mut heap = duke_gc::Heap::new();
    let s = heap.allocate_string("false".to_string());
    let mut out: Vec<u8> = Vec::new();
    let r = native_boolean_parseboolean(&[Slot::Reference(Some(s))], &mut heap, &mut out)
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(0));
}

// ---------------------------------------------------------------------------
// null-arm: String.valueOf(Object) null/nonnull
// ---------------------------------------------------------------------------

#[test]
fn native_string_value_of_object_null_returns_null_string() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let r = native_string_value_of_object(&[Slot::Reference(None)], &mut heap, &mut out)
        .unwrap()
        .unwrap();
    let ref_r = r.as_reference().unwrap();
    assert_eq!(
        heap.get(ref_r).unwrap().string_value.as_deref(),
        Some("null")
    );
}

#[test]
fn native_string_value_of_object_nonnull() {
    let mut heap = duke_gc::Heap::new();
    let obj = heap.allocate_string("hello".to_string());
    let mut out: Vec<u8> = Vec::new();
    let r = native_string_value_of_object(&[Slot::Reference(Some(obj))], &mut heap, &mut out)
        .unwrap()
        .unwrap();
    let ref_r = r.as_reference().unwrap();
    assert_eq!(
        heap.get(ref_r).unwrap().string_value.as_deref(),
        Some("hello")
    );
}

// ---------------------------------------------------------------------------
// null-arm: String.concat, String.replace(CharSequence), String.split
// ---------------------------------------------------------------------------

#[test]
fn native_string_concat_null_other_raises_npe() {
    let mut heap = duke_gc::Heap::new();
    let this = heap.allocate_string("hello".to_string());
    let mut out: Vec<u8> = Vec::new();
    let err = native_string_concat(
        &[Slot::Reference(Some(this)), Slot::Reference(None)],
        &mut heap,
        &mut out,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::NullPointerException));
}

#[test]
fn native_string_replace_charsequence_null_target_raises_npe() {
    let mut heap = duke_gc::Heap::new();
    let this = heap.allocate_string("hello".to_string());
    let rep = heap.allocate_string("x".to_string());
    let mut out: Vec<u8> = Vec::new();
    let err = native_string_replace_charsequence(
        &[
            Slot::Reference(Some(this)),
            Slot::Reference(None),
            Slot::Reference(Some(rep)),
        ],
        &mut heap,
        &mut out,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::NullPointerException));
}

#[test]
fn native_string_split_null_delimiter_raises_npe() {
    let mut heap = duke_gc::Heap::new();
    let this = heap.allocate_string("a,b".to_string());
    let mut out: Vec<u8> = Vec::new();
    let err = native_string_split(
        &[Slot::Reference(Some(this)), Slot::Reference(None)],
        &mut heap,
        &mut out,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::NullPointerException));
}

// ---------------------------------------------------------------------------
// native_math_min_double
// ---------------------------------------------------------------------------

#[test]
fn native_math_min_double_returns_smaller() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let r =
        native_math_min_double(&[Slot::Double(3.0), Slot::Double(1.5)], &mut heap, &mut out)
            .unwrap()
            .unwrap();
    assert!(matches!(r, Slot::Double(v) if (v - 1.5).abs() < 1e-9));
}

#[test]
fn native_math_min_double_returns_first_when_equal() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let r =
        native_math_min_double(&[Slot::Double(2.0), Slot::Double(2.0)], &mut heap, &mut out)
            .unwrap()
            .unwrap();
    assert!(matches!(r, Slot::Double(v) if (v - 2.0).abs() < 1e-9));
}

#[test]
fn native_math_floor_mod_int_keeps_divisor_sign() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let mut control = NativeControl::default();
    let r = native_math_floor_mod_int(
        &[Slot::Int(-1), Slot::Int(4)],
        &mut heap,
        &mut out,
        &mut control,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(3));
}

// ===========================================================================
// execute() — Ixor
// ===========================================================================

#[test]
fn execute_ixor_xors_bits() {
    let r = execute(
        &[
            (0, Instruction::Bipush(5_i8)),
            (2, Instruction::Bipush(3_i8)),
            (4, Instruction::Ixor),
            (5, Instruction::Ireturn),
        ],
        &[],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(6)); // 5 ^ 3 = 6
}

#[test]
fn execute_ixor_self_gives_zero() {
    let r = execute(
        &[
            (0, Instruction::Bipush(9_i8)),
            (2, Instruction::Bipush(9_i8)),
            (4, Instruction::Ixor),
            (5, Instruction::Ireturn),
        ],
        &[],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

// ===========================================================================
// execute() — Long bitwise: Land / Lor / Lxor / Lneg / Lshl masking
// ===========================================================================

#[test]
fn execute_land_selects_common_bits() {
    // 0b1010 & 0b1100 = 0b1000 = 8
    let r = execute(
        &[
            (0, Instruction::Lload0),
            (1, Instruction::Lload1),
            (2, Instruction::Land),
            (3, Instruction::Lreturn),
        ],
        &[],
        vec![Slot::Long(0b1010), Slot::Long(0b1100)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(8));
}

#[test]
fn execute_lor_combines_bits() {
    // 0b1010 | 0b0101 = 0b1111 = 15
    let r = execute(
        &[
            (0, Instruction::Lload0),
            (1, Instruction::Lload1),
            (2, Instruction::Lor),
            (3, Instruction::Lreturn),
        ],
        &[],
        vec![Slot::Long(0b1010), Slot::Long(0b0101)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(15));
}

#[test]
fn execute_lxor_flips_differing_bits() {
    // 0b1010 ^ 0b1100 = 0b0110 = 6
    let r = execute(
        &[
            (0, Instruction::Lload0),
            (1, Instruction::Lload1),
            (2, Instruction::Lxor),
            (3, Instruction::Lreturn),
        ],
        &[],
        vec![Slot::Long(0b1010), Slot::Long(0b1100)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(6));
}

#[test]
fn execute_lneg_negates_value() {
    let r = execute(
        &[
            (0, Instruction::Lload0),
            (1, Instruction::Lneg),
            (2, Instruction::Lreturn),
        ],
        &[],
        vec![Slot::Long(1)],
        4,
        1,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(-1));
}

#[test]
fn execute_lshl_masks_shift_amount_distinguishes_and_from_xor() {
    // shift by 65: 65 & 63 = 1 → 1L << 1 = 2
    // with ^ mutant: 65 ^ 63 = 64 → shift by 0 → 1
    let r = execute(
        &[
            (0, Instruction::Lload0),
            (1, Instruction::Iload1),
            (2, Instruction::Lshl),
            (3, Instruction::Lreturn),
        ],
        &[],
        vec![Slot::Long(1), Slot::Int(65)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(2));
}

// ===========================================================================
// execute() — Float arithmetic
// ===========================================================================

#[test]
fn execute_fadd_sums_floats() {
    let r = execute(
        &[
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Fadd),
            (3, Instruction::Freturn),
        ],
        &[],
        vec![Slot::Float(2.0), Slot::Float(3.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(r, Slot::Float(v) if (v - 5.0).abs() < 1e-6));
}

#[test]
fn execute_fsub_subtracts_floats() {
    let r = execute(
        &[
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Fsub),
            (3, Instruction::Freturn),
        ],
        &[],
        vec![Slot::Float(5.0), Slot::Float(3.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(r, Slot::Float(v) if (v - 2.0).abs() < 1e-6));
}

#[test]
fn execute_fmul_multiplies_floats() {
    let r = execute(
        &[
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Fmul),
            (3, Instruction::Freturn),
        ],
        &[],
        vec![Slot::Float(3.0), Slot::Float(4.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(r, Slot::Float(v) if (v - 12.0).abs() < 1e-6));
}

#[test]
fn execute_fdiv_divides_floats() {
    let r = execute(
        &[
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Fdiv),
            (3, Instruction::Freturn),
        ],
        &[],
        vec![Slot::Float(6.0), Slot::Float(2.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(r, Slot::Float(v) if (v - 3.0).abs() < 1e-6));
}

#[test]
fn execute_frem_float_remainder() {
    let r = execute(
        &[
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Frem),
            (3, Instruction::Freturn),
        ],
        &[],
        vec![Slot::Float(7.0), Slot::Float(3.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(r, Slot::Float(v) if (v - 1.0).abs() < 1e-6));
}

#[test]
fn execute_fneg_negates_float() {
    let r = execute(
        &[
            (0, Instruction::Fload0),
            (1, Instruction::Fneg),
            (2, Instruction::Freturn),
        ],
        &[],
        vec![Slot::Float(5.0)],
        4,
        1,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(r, Slot::Float(v) if (v + 5.0).abs() < 1e-6));
}

#[test]
fn execute_fcmpg_greater_gives_1() {
    let r = execute(
        &[
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Fcmpg),
            (3, Instruction::Ireturn),
        ],
        &[],
        vec![Slot::Float(3.0), Slot::Float(1.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_fcmpg_less_gives_minus1() {
    let r = execute(
        &[
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Fcmpg),
            (3, Instruction::Ireturn),
        ],
        &[],
        vec![Slot::Float(1.0), Slot::Float(3.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(-1));
}

#[test]
fn execute_fcmpg_equal_gives_0() {
    let r = execute(
        &[
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Fcmpg),
            (3, Instruction::Ireturn),
        ],
        &[],
        vec![Slot::Float(2.0), Slot::Float(2.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn execute_fcmpg_nan_gives_1() {
    let r = execute(
        &[
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Fcmpg),
            (3, Instruction::Ireturn),
        ],
        &[],
        vec![Slot::Float(f32::NAN), Slot::Float(1.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_fcmpl_nan_gives_minus1() {
    let r = execute(
        &[
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Fcmpl),
            (3, Instruction::Ireturn),
        ],
        &[],
        vec![Slot::Float(f32::NAN), Slot::Float(1.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(-1));
}

// ===========================================================================
// execute() — Double arithmetic
// ===========================================================================

#[test]
fn execute_dadd_sums_doubles() {
    let r = execute(
        &[
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Dadd),
            (3, Instruction::Dreturn),
        ],
        &[],
        vec![Slot::Double(2.0), Slot::Double(3.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(r, Slot::Double(v) if (v - 5.0).abs() < 1e-9));
}

#[test]
fn execute_dsub_subtracts_doubles() {
    let r = execute(
        &[
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Dsub),
            (3, Instruction::Dreturn),
        ],
        &[],
        vec![Slot::Double(5.0), Slot::Double(3.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(r, Slot::Double(v) if (v - 2.0).abs() < 1e-9));
}

#[test]
fn execute_dmul_multiplies_doubles() {
    let r = execute(
        &[
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Dmul),
            (3, Instruction::Dreturn),
        ],
        &[],
        vec![Slot::Double(3.0), Slot::Double(4.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(r, Slot::Double(v) if (v - 12.0).abs() < 1e-9));
}

#[test]
fn execute_ddiv_divides_doubles() {
    let r = execute(
        &[
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Ddiv),
            (3, Instruction::Dreturn),
        ],
        &[],
        vec![Slot::Double(6.0), Slot::Double(2.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(r, Slot::Double(v) if (v - 3.0).abs() < 1e-9));
}

#[test]
fn execute_drem_double_remainder() {
    let r = execute(
        &[
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Drem),
            (3, Instruction::Dreturn),
        ],
        &[],
        vec![Slot::Double(7.0), Slot::Double(3.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(r, Slot::Double(v) if (v - 1.0).abs() < 1e-9));
}

#[test]
fn execute_dneg_negates_double() {
    let r = execute(
        &[
            (0, Instruction::Dload0),
            (1, Instruction::Dneg),
            (2, Instruction::Dreturn),
        ],
        &[],
        vec![Slot::Double(5.0)],
        4,
        1,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(r, Slot::Double(v) if (v + 5.0).abs() < 1e-9));
}

#[test]
fn execute_dcmpg_greater_gives_1() {
    let r = execute(
        &[
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Dcmpg),
            (3, Instruction::Ireturn),
        ],
        &[],
        vec![Slot::Double(3.0), Slot::Double(1.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_dcmpg_less_gives_minus1() {
    let r = execute(
        &[
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Dcmpg),
            (3, Instruction::Ireturn),
        ],
        &[],
        vec![Slot::Double(1.0), Slot::Double(3.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(-1));
}

#[test]
fn execute_dcmpg_equal_gives_0() {
    let r = execute(
        &[
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Dcmpg),
            (3, Instruction::Ireturn),
        ],
        &[],
        vec![Slot::Double(2.0), Slot::Double(2.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn execute_dcmpg_nan_gives_1() {
    let r = execute(
        &[
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Dcmpg),
            (3, Instruction::Ireturn),
        ],
        &[],
        vec![Slot::Double(f64::NAN), Slot::Double(1.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_dcmpl_nan_gives_minus1() {
    let r = execute(
        &[
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Dcmpl),
            (3, Instruction::Ireturn),
        ],
        &[],
        vec![Slot::Double(f64::NAN), Slot::Double(1.0)],
        4,
        2,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(-1));
}

// ===========================================================================
// execute() — Conditional branches
// Pattern: push condition, Ifxx(offset=4) at PC=1 → target=5
// ===========================================================================

#[test]
fn execute_ifne_taken_when_nonzero() {
    let r = execute(
        &[
            (0, Instruction::Bipush(5_i8)),
            (2, Instruction::Ifne(4)),
            (4, Instruction::Iconst0),
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst1),
            (7, Instruction::Ireturn),
        ],
        &[],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_ifne_not_taken_when_zero() {
    let r = execute(
        &[
            (0, Instruction::Iconst0),
            (1, Instruction::Ifne(4)),
            (3, Instruction::Iconst0),
            (4, Instruction::Ireturn),
            (5, Instruction::Iconst1),
            (6, Instruction::Ireturn),
        ],
        &[],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn execute_iflt_taken_when_negative() {
    let r = execute(
        &[
            (0, Instruction::IconstM1),
            (1, Instruction::Iflt(4)),
            (3, Instruction::Iconst0),
            (4, Instruction::Ireturn),
            (5, Instruction::Iconst1),
            (6, Instruction::Ireturn),
        ],
        &[],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_iflt_not_taken_when_zero() {
    let r = execute(
        &[
            (0, Instruction::Iconst0),
            (1, Instruction::Iflt(4)),
            (3, Instruction::Iconst0),
            (4, Instruction::Ireturn),
            (5, Instruction::Iconst1),
            (6, Instruction::Ireturn),
        ],
        &[],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn execute_ifgt_taken_when_positive() {
    let r = execute(
        &[
            (0, Instruction::Iconst1),
            (1, Instruction::Ifgt(4)),
            (3, Instruction::Iconst0),
            (4, Instruction::Ireturn),
            (5, Instruction::Iconst1),
            (6, Instruction::Ireturn),
        ],
        &[],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_ifgt_not_taken_when_zero() {
    let r = execute(
        &[
            (0, Instruction::Iconst0),
            (1, Instruction::Ifgt(4)),
            (3, Instruction::Iconst0),
            (4, Instruction::Ireturn),
            (5, Instruction::Iconst1),
            (6, Instruction::Ireturn),
        ],
        &[],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn execute_ifle_taken_when_negative() {
    let r = execute(
        &[
            (0, Instruction::IconstM1),
            (1, Instruction::Ifle(4)),
            (3, Instruction::Iconst0),
            (4, Instruction::Ireturn),
            (5, Instruction::Iconst1),
            (6, Instruction::Ireturn),
        ],
        &[],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_ifle_taken_when_zero() {
    let r = execute(
        &[
            (0, Instruction::Iconst0),
            (1, Instruction::Ifle(4)),
            (3, Instruction::Iconst0),
            (4, Instruction::Ireturn),
            (5, Instruction::Iconst1),
            (6, Instruction::Ireturn),
        ],
        &[],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_ifle_not_taken_when_positive() {
    let r = execute(
        &[
            (0, Instruction::Iconst1),
            (1, Instruction::Ifle(4)),
            (3, Instruction::Iconst0),
            (4, Instruction::Ireturn),
            (5, Instruction::Iconst1),
            (6, Instruction::Ireturn),
        ],
        &[],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn execute_ifnull_taken_when_null() {
    let r = execute(
        &[
            (0, Instruction::AconstNull),
            (1, Instruction::Ifnull(4)),
            (3, Instruction::Iconst0),
            (4, Instruction::Ireturn),
            (5, Instruction::Iconst1),
            (6, Instruction::Ireturn),
        ],
        &[],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_ifnull_not_taken_when_nonnull() {
    let r = execute(
        &[
            (0, Instruction::Aload0),
            (1, Instruction::Ifnull(4)),
            (3, Instruction::Iconst1),
            (4, Instruction::Ireturn),
            (5, Instruction::Iconst0),
            (6, Instruction::Ireturn),
        ],
        &[],
        vec![Slot::Reference(Some(0))],
        4,
        1,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_ifnonnull_taken_when_nonnull() {
    let r = execute(
        &[
            (0, Instruction::Aload0),
            (1, Instruction::Ifnonnull(4)),
            (3, Instruction::Iconst0),
            (4, Instruction::Ireturn),
            (5, Instruction::Iconst1),
            (6, Instruction::Ireturn),
        ],
        &[],
        vec![Slot::Reference(Some(42))],
        4,
        1,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_ifnonnull_not_taken_when_null() {
    let r = execute(
        &[
            (0, Instruction::AconstNull),
            (1, Instruction::Ifnonnull(4)),
            (3, Instruction::Iconst0),
            (4, Instruction::Ireturn),
            (5, Instruction::Iconst1),
            (6, Instruction::Ireturn),
        ],
        &[],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn execute_ificmplt_taken_when_a_less_than_b() {
    let r = execute(
        &[
            (0, Instruction::Iconst2),
            (1, Instruction::Iconst3),
            (2, Instruction::IfIcmplt(4)),
            (4, Instruction::Iconst0),
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst1),
            (7, Instruction::Ireturn),
        ],
        &[],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1)); // 2 < 3 → taken
}

#[test]
fn execute_ificmplt_not_taken_when_equal() {
    let r = execute(
        &[
            (0, Instruction::Iconst3),
            (1, Instruction::Iconst3),
            (2, Instruction::IfIcmplt(4)),
            (4, Instruction::Iconst0),
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst1),
            (7, Instruction::Ireturn),
        ],
        &[],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0)); // 3 < 3 is false → not taken
}

// ===========================================================================
// execute() — Newarray Long/Float/Double initializes correct slot types
// ===========================================================================

#[test]
fn execute_newarray_long_initializes_long_zero() {
    use duke_bytecode::instruction::ArrayType;
    let r = execute(
        &[
            (0, Instruction::Iconst1),
            (1, Instruction::Newarray(ArrayType::Long)),
            (3, Instruction::Dup),
            (4, Instruction::Iconst0),
            (5, Instruction::Laload),
            (6, Instruction::Lreturn),
        ],
        &[],
        vec![],
        8,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(0));
}

#[test]
fn execute_newarray_float_initializes_float_zero() {
    use duke_bytecode::instruction::ArrayType;
    let r = execute(
        &[
            (0, Instruction::Iconst1),
            (1, Instruction::Newarray(ArrayType::Float)),
            (3, Instruction::Dup),
            (4, Instruction::Iconst0),
            (5, Instruction::Faload),
            (6, Instruction::Freturn),
        ],
        &[],
        vec![],
        8,
        0,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(r, Slot::Float(v) if v == 0.0));
}

#[test]
fn execute_newarray_double_initializes_double_zero() {
    use duke_bytecode::instruction::ArrayType;
    let r = execute(
        &[
            (0, Instruction::Iconst1),
            (1, Instruction::Newarray(ArrayType::Double)),
            (3, Instruction::Dup),
            (4, Instruction::Iconst0),
            (5, Instruction::Daload),
            (6, Instruction::Dreturn),
        ],
        &[],
        vec![],
        8,
        0,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(r, Slot::Double(v) if v == 0.0));
}

// ===========================================================================
// execute() — Long/Float/Double array store/load roundtrip + bounds
// ===========================================================================

#[test]
fn execute_lastore_and_laload_roundtrip() {
    use duke_bytecode::instruction::ArrayType;
    // Create long[2], store Lconst1 at index 0, load and return it.
    let r = execute(
        &[
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Long)),
            (3, Instruction::Dup),
            (4, Instruction::Iconst0),
            (5, Instruction::Lconst1),
            (6, Instruction::Lastore),
            (7, Instruction::Iconst0),
            (8, Instruction::Laload),
            (9, Instruction::Lreturn),
        ],
        &[],
        vec![],
        8,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(1));
}

#[test]
fn execute_laload_out_of_bounds_raises_error() {
    use duke_bytecode::instruction::ArrayType;
    let err = execute(
        &[
            (0, Instruction::Iconst1),
            (1, Instruction::Newarray(ArrayType::Long)),
            (3, Instruction::Iconst2),
            (4, Instruction::Laload),
            (5, Instruction::Lreturn),
        ],
        &[],
        vec![],
        4,
        0,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

#[test]
fn execute_fastore_and_faload_roundtrip() {
    use duke_bytecode::instruction::ArrayType;
    let r = execute(
        &[
            (0, Instruction::Iconst1),
            (1, Instruction::Newarray(ArrayType::Float)),
            (3, Instruction::Dup),
            (4, Instruction::Iconst0),
            (5, Instruction::Fconst2),
            (6, Instruction::Fastore),
            (7, Instruction::Iconst0),
            (8, Instruction::Faload),
            (9, Instruction::Freturn),
        ],
        &[],
        vec![],
        8,
        0,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(r, Slot::Float(v) if (v - 2.0).abs() < 1e-6));
}

#[test]
fn execute_dastore_and_daload_roundtrip() {
    use duke_bytecode::instruction::ArrayType;
    let r = execute(
        &[
            (0, Instruction::Iconst1),
            (1, Instruction::Newarray(ArrayType::Double)),
            (3, Instruction::Dup),
            (4, Instruction::Iconst0),
            (5, Instruction::Dconst1),
            (6, Instruction::Dastore),
            (7, Instruction::Iconst0),
            (8, Instruction::Daload),
            (9, Instruction::Dreturn),
        ],
        &[],
        vec![],
        8,
        0,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(r, Slot::Double(v) if (v - 1.0).abs() < 1e-9));
}

#[test]
fn execute_faload_out_of_bounds_raises_error() {
    use duke_bytecode::instruction::ArrayType;
    let err = execute(
        &[
            (0, Instruction::Iconst1),
            (1, Instruction::Newarray(ArrayType::Float)),
            (3, Instruction::Sipush(5_i16)),
            (6, Instruction::Faload),
            (7, Instruction::Freturn),
        ],
        &[],
        vec![],
        4,
        0,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

#[test]
fn execute_daload_out_of_bounds_raises_error() {
    use duke_bytecode::instruction::ArrayType;
    let err = execute(
        &[
            (0, Instruction::Iconst1),
            (1, Instruction::Newarray(ArrayType::Double)),
            (3, Instruction::Sipush(10_i16)),
            (6, Instruction::Daload),
            (7, Instruction::Dreturn),
        ],
        &[],
        vec![],
        4,
        0,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ===========================================================================
// FramePool: acquire / release / cap-at-256
// ===========================================================================

#[test]
fn frame_pool_acquire_from_empty_gives_empty_vecs() {
    let mut pool = FramePool::new();
    let (locals, stack) = pool.acquire();
    assert!(locals.is_empty());
    assert!(stack.is_empty());
}

#[test]
fn frame_pool_release_and_reacquire_returns_pooled_bufs() {
    let mut pool = FramePool::new();
    let locals = vec![Slot::Int(1), Slot::Int(2)];
    let stack = vec![];
    pool.release(locals, stack);
    let (locals2, stack2) = pool.acquire();
    assert_eq!(locals2.len(), 2);
    assert!(stack2.is_empty());
    // Pool should be empty again
    let (locals3, _) = pool.acquire();
    assert!(locals3.is_empty());
}

#[test]
fn frame_pool_release_caps_at_256() {
    let mut pool = FramePool::new();
    for i in 0..300_i32 {
        pool.release(vec![Slot::Int(i)], vec![]);
    }
    assert_eq!(pool.free.len(), 256);
}

#[test]
fn frame_pool_release_below_256_keeps_all() {
    let mut pool = FramePool::new();
    for i in 0..10_i32 {
        pool.release(vec![Slot::Int(i)], vec![]);
    }
    assert_eq!(pool.free.len(), 10);
}

// ===========================================================================
// Helper functions: parse_arg_count / parse_arg_types
// ===========================================================================

#[test]
fn parse_arg_count_primitives() {
    assert_eq!(parse_arg_count("(IZB)V"), 3);
    assert_eq!(parse_arg_count("(JFDS)V"), 4);
    assert_eq!(parse_arg_count("()V"), 0);
    assert_eq!(parse_arg_count("(I)I"), 1);
}

#[test]
fn parse_arg_count_object_type_counts_once() {
    assert_eq!(parse_arg_count("(Ljava/lang/String;)V"), 1);
    assert_eq!(parse_arg_count("(Ljava/lang/String;I)V"), 2);
}

#[test]
fn parse_arg_count_array_types() {
    assert_eq!(parse_arg_count("([I)V"), 1);
    assert_eq!(parse_arg_count("([Ljava/lang/String;)V"), 1);
    assert_eq!(parse_arg_count("([[I)V"), 1); // 2-D array = 1 slot
    assert_eq!(parse_arg_count("([I[Z)V"), 2);
}

#[test]
fn parse_arg_count_malformed() {
    assert_eq!(parse_arg_count("invalid"), 0);
    assert_eq!(parse_arg_count("("), 0);
    assert_eq!(parse_arg_count(")"), 0);
    assert_eq!(parse_arg_count("(I"), 0);
}

#[test]
fn parse_arg_types_primitives() {
    assert_eq!(parse_arg_types("(I)V"), vec!['I']);
    assert_eq!(parse_arg_types("(IZB)V"), vec!['I', 'Z', 'B']);
    assert_eq!(parse_arg_types("()V"), Vec::<char>::new());
}

#[test]
fn parse_arg_types_object_and_array() {
    assert_eq!(parse_arg_types("(Ljava/lang/String;)V"), vec!['L']);
    assert_eq!(parse_arg_types("([I)V"), vec!['[']);
    assert_eq!(parse_arg_types("([Ljava/lang/String;)V"), vec!['[']);
    assert_eq!(
        parse_arg_types("(ILjava/lang/String;[I)V"),
        vec!['I', 'L', '[']
    );
}

#[test]
fn parse_arg_types_malformed() {
    assert_eq!(parse_arg_types("invalid"), vec![]);
    assert_eq!(parse_arg_types("("), vec![]);
    assert_eq!(parse_arg_types(")"), vec![]);
    assert_eq!(parse_arg_types("(I"), vec![]);
}

// ===========================================================================
// Helper functions: default_slot_for_descriptor
// ===========================================================================

#[test]
fn default_slot_for_descriptor_long_is_long_zero() {
    assert_eq!(default_slot_for_descriptor("J"), Slot::Long(0));
}

#[test]
fn default_slot_for_descriptor_float_is_float_zero() {
    assert!(matches!(default_slot_for_descriptor("F"), Slot::Float(v) if v == 0.0));
}

#[test]
fn default_slot_for_descriptor_double_is_double_zero() {
    assert!(matches!(default_slot_for_descriptor("D"), Slot::Double(v) if v == 0.0));
}

#[test]
fn default_slot_for_descriptor_reference_types_are_null() {
    assert!(matches!(
        default_slot_for_descriptor("Ljava/lang/String;"),
        Slot::Reference(None)
    ));
    assert!(matches!(
        default_slot_for_descriptor("[I"),
        Slot::Reference(None)
    ));
}

#[test]
fn default_slot_for_descriptor_int_and_others_are_int_zero() {
    assert_eq!(default_slot_for_descriptor("I"), Slot::Int(0));
    assert_eq!(default_slot_for_descriptor("Z"), Slot::Int(0));
    assert_eq!(default_slot_for_descriptor("B"), Slot::Int(0));
}

// ===========================================================================
// Helper function: resolve_cp_string
// ===========================================================================

#[test]
fn resolve_cp_string_from_string_entry() {
    use duke_classfile::types::CpIndex;
    let cp = vec![
        None,
        Some(CpEntry::Utf8("hello".to_string())),
        Some(CpEntry::String {
            string_index: CpIndex(1),
        }),
    ];
    assert_eq!(resolve_cp_string(&cp, 2).unwrap(), "hello");
}

#[test]
fn resolve_cp_string_from_utf8_entry() {
    use duke_classfile::types::CpIndex;
    let cp = vec![
        None,
        Some(CpEntry::Utf8("world".to_string())),
        Some(CpEntry::String {
            string_index: CpIndex(1),
        }),
    ];
    assert_eq!(resolve_cp_string(&cp, 1).unwrap(), "world");
}

#[test]
fn resolve_cp_string_missing_utf8_raises_error() {
    use duke_classfile::types::CpIndex;
    let cp = vec![
        None,
        None, // missing Utf8
        Some(CpEntry::String {
            string_index: CpIndex(1),
        }),
    ];
    assert!(resolve_cp_string(&cp, 2).is_err());
}

#[test]
fn resolve_cp_string_out_of_bounds_raises_error() {
    let cp: Vec<Option<CpEntry>> = vec![None];
    assert!(resolve_cp_string(&cp, 99).is_err());
}

// ===========================================================================
// Helper function: is_assignable_from — interface walk
// ===========================================================================

fn make_simple_loader() -> duke_loader::DirectoryLoader {
    duke_loader::DirectoryLoader::new(std::path::Path::new("."))
}

#[test]
fn is_assignable_from_same_class_is_true() {
    let mut registry = ClassRegistry::new();
    let loader = make_simple_loader();
    assert!(is_assignable_from(
        &mut registry,
        &loader,
        "Foo",
        "Foo",
        None
    ));
}

#[test]
fn is_assignable_from_object_is_always_true() {
    let mut registry = ClassRegistry::new();
    let loader = make_simple_loader();
    assert!(is_assignable_from(
        &mut registry,
        &loader,
        "Anything",
        "java/lang/Object",
        None
    ));
}

#[test]
fn is_assignable_from_via_direct_interface_is_true() {
    let mut registry = ClassRegistry::new();
    registry.register(ClassContext {
        class_name: "MyClass".to_string(),
        super_class: None,
        interfaces: vec!["MyInterface".to_string()],
        constant_pool: vec![],
        methods: vec![],
        fields: vec![],
        static_fields: vec![],
        instance_field_count: 0,
        bootstrap_methods: vec![],
    });
    let loader = make_simple_loader();
    assert!(is_assignable_from(
        &mut registry,
        &loader,
        "MyClass",
        "MyInterface",
        None
    ));
}

#[test]
fn is_assignable_from_unrelated_class_is_false() {
    let mut registry = ClassRegistry::new();
    registry.register(ClassContext {
        class_name: "MyClass".to_string(),
        super_class: None,
        interfaces: vec!["InterfaceA".to_string()],
        constant_pool: vec![],
        methods: vec![],
        fields: vec![],
        static_fields: vec![],
        instance_field_count: 0,
        bootstrap_methods: vec![],
    });
    let loader = make_simple_loader();
    assert!(!is_assignable_from(
        &mut registry,
        &loader,
        "MyClass",
        "InterfaceB",
        None
    ));
}

// ===========================================================================
// Helper function: find_exception_handler — end_pc boundary
// ===========================================================================

#[test]
fn find_exception_handler_catches_within_range() {
    let table = vec![ExceptionEntry {
        start_pc: 0,
        end_pc: 10,
        handler_pc: 20,
        catch_type: None, // catch-all
    }];
    let mut registry = ClassRegistry::new();
    let loader = make_simple_loader();
    assert_eq!(
        find_exception_handler(
            &table,
            5,
            "java/lang/Exception",
            "HandlerOwner",
            &mut registry,
            &loader,
        ),
        Some(20)
    );
}

#[test]
fn find_exception_handler_excludes_at_end_pc() {
    // end_pc is exclusive per JVM spec
    let table = vec![ExceptionEntry {
        start_pc: 0,
        end_pc: 10,
        handler_pc: 20,
        catch_type: None,
    }];
    let mut registry = ClassRegistry::new();
    let loader = make_simple_loader();
    assert_eq!(
        find_exception_handler(
            &table,
            10,
            "java/lang/Exception",
            "HandlerOwner",
            &mut registry,
            &loader,
        ),
        None
    );
    assert_eq!(
        find_exception_handler(
            &table,
            9,
            "java/lang/Exception",
            "HandlerOwner",
            &mut registry,
            &loader,
        ),
        Some(20)
    );
}

// ===========================================================================
// Helper function: field_slot_idx — single and hierarchical classes
// ===========================================================================

fn make_two_class_registry() -> ClassRegistry {
    let mut registry = ClassRegistry::new();
    registry.register(ClassContext {
        class_name: "Base".to_string(),
        super_class: None,
        interfaces: vec![],
        constant_pool: vec![],
        methods: vec![],
        fields: vec![
            FieldEntry {
                name: "x".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "y".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
        ],
        static_fields: vec![],
        instance_field_count: 2,
        bootstrap_methods: vec![],
    });
    registry.register(ClassContext {
        class_name: "Child".to_string(),
        super_class: Some("Base".to_string()),
        interfaces: vec![],
        constant_pool: vec![],
        methods: vec![],
        fields: vec![FieldEntry {
            name: "z".to_string(),
            descriptor: "I".to_string(),
            is_static: false,
        }],
        static_fields: vec![],
        instance_field_count: 1,
        bootstrap_methods: vec![],
    });
    registry
}

#[test]
fn field_slot_idx_finds_first_field_at_slot_0() {
    let registry = make_two_class_registry();
    assert_eq!(field_slot_idx(&registry, "Base", "x").unwrap(), 0);
}

#[test]
fn field_slot_idx_finds_second_field_at_slot_1() {
    let registry = make_two_class_registry();
    assert_eq!(field_slot_idx(&registry, "Base", "y").unwrap(), 1);
}

#[test]
fn field_slot_idx_finds_child_field_after_parent_fields() {
    let registry = make_two_class_registry();
    // Base has 2 fields (x at 0, y at 1); Child adds z → slot 2
    assert_eq!(field_slot_idx(&registry, "Child", "z").unwrap(), 2);
}

#[test]
fn field_slot_idx_missing_field_raises_error() {
    let registry = make_two_class_registry();
    assert!(field_slot_idx(&registry, "Base", "nonexistent").is_err());
}

// ===========================================================================
// native_sb_init_string: null arg falls back to empty string
// ===========================================================================

#[test]
fn native_sb_init_string_null_arg_gives_empty_buffer() {
    let mut heap = duke_gc::Heap::new();
    let this_ref = heap.allocate("java/lang/StringBuilder".to_string(), 1);
    let mut out: Vec<u8> = Vec::new();
    native_sb_init_string(
        &[
            Slot::Reference(Some(this_ref)),
            Slot::Reference(None), // null arg
        ],
        &mut heap,
        &mut out,
    )
    .unwrap();
    assert_eq!(
        heap.get(this_ref).unwrap().string_value.as_deref(),
        Some("")
    );
}

// ===========================================================================
// native_sb_append_string: null appends "null" literal
// ===========================================================================

#[test]
fn native_sb_append_string_null_arg_appends_null_literal() {
    let mut heap = duke_gc::Heap::new();
    let this_ref = heap.allocate("java/lang/StringBuilder".to_string(), 1);
    heap.get_mut(this_ref).unwrap().string_value = Some("hi".to_string());
    let mut out: Vec<u8> = Vec::new();
    native_sb_append_string(
        &[Slot::Reference(Some(this_ref)), Slot::Reference(None)],
        &mut heap,
        &mut out,
    )
    .unwrap();
    assert_eq!(
        heap.get(this_ref).unwrap().string_value.as_deref(),
        Some("hinull")
    );
}

// ===========================================================================
// native_sb_append_char: Slot::Int(v) arm
// ===========================================================================

#[test]
fn native_sb_append_char_appends_unicode_char() {
    let mut heap = duke_gc::Heap::new();
    let this_ref = heap.allocate("java/lang/StringBuilder".to_string(), 1);
    heap.get_mut(this_ref).unwrap().string_value = Some(String::new());
    let mut out: Vec<u8> = Vec::new();
    native_sb_append_char(
        &[
            Slot::Reference(Some(this_ref)),
            Slot::Int('A' as i32), // 65
        ],
        &mut heap,
        &mut out,
    )
    .unwrap();
    assert_eq!(
        heap.get(this_ref).unwrap().string_value.as_deref(),
        Some("A")
    );
}

// ===========================================================================
// native_arraylist_get: index arithmetic (idx + 1)
// ===========================================================================

#[test]
fn native_arraylist_get_index_arithmetic_retrieves_correct_element() {
    let mut heap = duke_gc::Heap::new();
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 4);
    let v0 = heap.allocate_string("first".to_string());
    let v1 = heap.allocate_string("second".to_string());
    let v2 = heap.allocate_string("third".to_string());
    {
        let obj = heap.get_mut(list_ref).unwrap();
        obj.fields[0] = Slot::Int(3); // size
        obj.fields[1] = Slot::Reference(Some(v0));
        obj.fields[2] = Slot::Reference(Some(v1));
        obj.fields[3] = Slot::Reference(Some(v2));
    }
    let mut out: Vec<u8> = Vec::new();
    // get(1) should return fields[2] = v1 (index 1+1=2)
    let result = native_arraylist_get(
        &[Slot::Reference(Some(list_ref)), Slot::Int(1)],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    if let Slot::Reference(Some(r)) = result {
        assert_eq!(heap.get(r).unwrap().string_value.as_deref(), Some("second"));
    } else {
        panic!("expected reference, got {result:?}");
    }
}

#[test]
fn native_arraylist_get_index_zero_returns_first_element() {
    let mut heap = duke_gc::Heap::new();
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 2);
    let v0 = heap.allocate_string("alpha".to_string());
    {
        let obj = heap.get_mut(list_ref).unwrap();
        obj.fields[0] = Slot::Int(1);
        obj.fields[1] = Slot::Reference(Some(v0));
    }
    let mut out: Vec<u8> = Vec::new();
    let result = native_arraylist_get(
        &[Slot::Reference(Some(list_ref)), Slot::Int(0)],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    if let Slot::Reference(Some(r)) = result {
        assert_eq!(heap.get(r).unwrap().string_value.as_deref(), Some("alpha"));
    } else {
        panic!("expected reference");
    }
}

// ===========================================================================
// native_arrays_fill_object: reference value arm
// ===========================================================================

#[test]
fn native_arrays_fill_object_fills_with_reference() {
    let mut heap = duke_gc::Heap::new();
    let arr_ref = heap.allocate("[Ljava/lang/Object;".to_string(), 3);
    let val_ref = heap.allocate_string("x".to_string());
    let mut out: Vec<u8> = Vec::new();
    native_arrays_fill_object(
        &[
            Slot::Reference(Some(arr_ref)),
            Slot::Reference(Some(val_ref)),
        ],
        &mut heap,
        &mut out,
    )
    .unwrap();
    let fields = &heap.get(arr_ref).unwrap().fields;
    assert!(
        fields
            .iter()
            .all(|s| matches!(s, Slot::Reference(Some(r)) if *r == val_ref))
    );
}

// ===========================================================================
// native_arrays_copyof_int: negative length raises NegativeArraySize
// ===========================================================================

#[test]
fn native_arrays_copyof_int_negative_length_raises_nsa() {
    let mut heap = duke_gc::Heap::new();
    let src = heap.allocate("[I".to_string(), 3);
    let mut out: Vec<u8> = Vec::new();
    let err = native_arrays_copyof_int(
        &[Slot::Reference(Some(src)), Slot::Int(-1)],
        &mut heap,
        &mut out,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::NegativeArraySize { size: -1 }));
}

#[test]
fn native_arrays_copyof_int_truncates_when_shorter() {
    let mut heap = duke_gc::Heap::new();
    let src = heap.allocate("[I".to_string(), 3);
    {
        let obj = heap.get_mut(src).unwrap();
        obj.fields[0] = Slot::Int(10);
        obj.fields[1] = Slot::Int(20);
        obj.fields[2] = Slot::Int(30);
    }
    let mut out: Vec<u8> = Vec::new();
    let result = native_arrays_copyof_int(
        &[Slot::Reference(Some(src)), Slot::Int(2)],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    if let Slot::Reference(Some(dst_ref)) = result {
        let fields = &heap.get(dst_ref).unwrap().fields;
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0], Slot::Int(10));
        assert_eq!(fields[1], Slot::Int(20));
    } else {
        panic!("expected reference");
    }
}

// ===========================================================================
// native_arrays_copyof_object: negative length raises NegativeArraySize,
//   and extending with null Reference
// ===========================================================================

#[test]
fn native_arrays_copyof_object_negative_length_raises_nsa() {
    let mut heap = duke_gc::Heap::new();
    let src = heap.allocate("[Ljava/lang/Object;".to_string(), 1);
    let mut out: Vec<u8> = Vec::new();
    let err = native_arrays_copyof_object(
        &[Slot::Reference(Some(src)), Slot::Int(-2)],
        &mut heap,
        &mut out,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::NegativeArraySize { size: -2 }));
}

#[test]
fn native_arrays_copyof_object_extends_with_null() {
    let mut heap = duke_gc::Heap::new();
    let v = heap.allocate_string("item".to_string());
    let src = heap.allocate("[Ljava/lang/Object;".to_string(), 1);
    heap.get_mut(src).unwrap().fields[0] = Slot::Reference(Some(v));
    let mut out: Vec<u8> = Vec::new();
    let result = native_arrays_copyof_object(
        &[Slot::Reference(Some(src)), Slot::Int(3)],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    if let Slot::Reference(Some(dst_ref)) = result {
        let fields = &heap.get(dst_ref).unwrap().fields;
        assert_eq!(fields.len(), 3);
        assert!(matches!(fields[0], Slot::Reference(Some(_))));
        assert!(matches!(fields[1], Slot::Reference(None)));
        assert!(matches!(fields[2], Slot::Reference(None)));
    } else {
        panic!("expected reference");
    }
}

// ===========================================================================
// slots_equal: null-null, null-nonnull, string equality, class equality
// ===========================================================================

#[test]
fn slots_equal_null_null_is_true() {
    let heap = duke_gc::Heap::new();
    assert!(slots_equal(
        &Slot::Reference(None),
        &Slot::Reference(None),
        &heap
    ));
}

#[test]
fn slots_equal_null_nonnull_is_false() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate_string("x".to_string());
    assert!(!slots_equal(
        &Slot::Reference(None),
        &Slot::Reference(Some(r)),
        &heap
    ));
    assert!(!slots_equal(
        &Slot::Reference(Some(r)),
        &Slot::Reference(None),
        &heap
    ));
}

#[test]
fn slots_equal_strings_compared_by_value() {
    let mut heap = duke_gc::Heap::new();
    let r1 = heap.allocate_string("hello".to_string());
    let r2 = heap.allocate_string("hello".to_string()); // different ref, same value
    let r3 = heap.allocate_string("world".to_string());
    assert!(slots_equal(
        &Slot::Reference(Some(r1)),
        &Slot::Reference(Some(r2)),
        &heap
    ));
    assert!(!slots_equal(
        &Slot::Reference(Some(r1)),
        &Slot::Reference(Some(r3)),
        &heap
    ));
}

#[test]
fn slots_equal_boxed_integer_same_value_is_true() {
    let mut heap = duke_gc::Heap::new();
    let r1 = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r1).unwrap().fields[0] = Slot::Int(42);
    let r2 = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r2).unwrap().fields[0] = Slot::Int(42);
    assert!(slots_equal(
        &Slot::Reference(Some(r1)),
        &Slot::Reference(Some(r2)),
        &heap
    ));
}

#[test]
fn slots_equal_arbitrary_same_class_objects_use_identity() {
    let mut heap = duke_gc::Heap::new();
    let r1 = heap.allocate("java/net/URL".to_string(), 1);
    let r2 = heap.allocate("java/net/URL".to_string(), 1);
    assert!(!slots_equal(
        &Slot::Reference(Some(r1)),
        &Slot::Reference(Some(r2)),
        &heap
    ));
}

#[test]
fn slots_equal_different_classes_is_false() {
    let mut heap = duke_gc::Heap::new();
    let r1 = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r1).unwrap().fields[0] = Slot::Int(1);
    let r2 = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r2).unwrap().fields[0] = Slot::Int(1);
    assert!(!slots_equal(
        &Slot::Reference(Some(r1)),
        &Slot::Reference(Some(r2)),
        &heap
    ));
}

// ===========================================================================
// native_hashmap_put/get/contains_key/remove/get_or_default with 2 entries
// (exercises the i += 2 loop arithmetic)
// ===========================================================================

fn make_hashmap_with_two_string_entries() -> (duke_gc::Heap, u64, u64, u64, u64, u64) {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let hm_ref = heap.allocate("java/util/HashMap".to_string(), 1);
    native_hashmap_init(&[Slot::Reference(Some(hm_ref))], &mut heap, &mut out).unwrap();
    let k1 = heap.allocate_string("key1".to_string());
    let v1 = heap.allocate_string("val1".to_string());
    let k2 = heap.allocate_string("key2".to_string());
    let v2 = heap.allocate_string("val2".to_string());
    native_hashmap_put(
        &[
            Slot::Reference(Some(hm_ref)),
            Slot::Reference(Some(k1)),
            Slot::Reference(Some(v1)),
        ],
        &mut heap,
        &mut out,
    )
    .unwrap();
    native_hashmap_put(
        &[
            Slot::Reference(Some(hm_ref)),
            Slot::Reference(Some(k2)),
            Slot::Reference(Some(v2)),
        ],
        &mut heap,
        &mut out,
    )
    .unwrap();
    (heap, hm_ref, k1, v1, k2, v2)
}

#[test]
fn native_hashmap_get_finds_second_entry() {
    let (mut heap, hm_ref, _k1, _v1, k2, v2) = make_hashmap_with_two_string_entries();
    let mut out: Vec<u8> = Vec::new();
    let result = native_hashmap_get(
        &[Slot::Reference(Some(hm_ref)), Slot::Reference(Some(k2))],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    if let Slot::Reference(Some(r)) = result {
        assert_eq!(
            heap.get(r).unwrap().string_value,
            heap.get(v2).unwrap().string_value
        );
    } else {
        panic!("expected reference, got {result:?}");
    }
}

#[test]
fn native_hashmap_get_finds_first_entry() {
    let (mut heap, hm_ref, k1, v1, _k2, _v2) = make_hashmap_with_two_string_entries();
    let mut out: Vec<u8> = Vec::new();
    let result = native_hashmap_get(
        &[Slot::Reference(Some(hm_ref)), Slot::Reference(Some(k1))],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    if let Slot::Reference(Some(r)) = result {
        assert_eq!(
            heap.get(r).unwrap().string_value,
            heap.get(v1).unwrap().string_value
        );
    } else {
        panic!("expected reference, got {result:?}");
    }
}

#[test]
fn native_hashmap_contains_key_finds_second_entry() {
    let (mut heap, hm_ref, _k1, _v1, k2, _v2) = make_hashmap_with_two_string_entries();
    let mut out: Vec<u8> = Vec::new();
    let r = native_hashmap_contains_key(
        &[Slot::Reference(Some(hm_ref)), Slot::Reference(Some(k2))],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn native_hashmap_contains_key_absent_key_is_false() {
    let (mut heap, hm_ref, _k1, _v1, _k2, _v2) = make_hashmap_with_two_string_entries();
    let other_key = heap.allocate_string("absent".to_string());
    let mut out: Vec<u8> = Vec::new();
    let r = native_hashmap_contains_key(
        &[
            Slot::Reference(Some(hm_ref)),
            Slot::Reference(Some(other_key)),
        ],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn native_hashmap_remove_second_entry_decrements_size() {
    let (mut heap, hm_ref, _k1, _v1, k2, v2) = make_hashmap_with_two_string_entries();
    let mut out: Vec<u8> = Vec::new();
    let old = native_hashmap_remove(
        &[Slot::Reference(Some(hm_ref)), Slot::Reference(Some(k2))],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    if let Slot::Reference(Some(r)) = old {
        assert_eq!(
            heap.get(r).unwrap().string_value,
            heap.get(v2).unwrap().string_value
        );
    } else {
        panic!("expected reference, got {old:?}");
    }
    // Size should now be 1
    let size = native_hashmap_size(&[Slot::Reference(Some(hm_ref))], &mut heap, &mut out)
        .unwrap()
        .unwrap();
    assert_eq!(size, Slot::Int(1));
}

#[test]
fn native_hashmap_remove_first_entry_leaves_second_findable() {
    let (mut heap, hm_ref, k1, _v1, k2, v2) = make_hashmap_with_two_string_entries();
    let mut out: Vec<u8> = Vec::new();
    // Remove first entry
    native_hashmap_remove(
        &[Slot::Reference(Some(hm_ref)), Slot::Reference(Some(k1))],
        &mut heap,
        &mut out,
    )
    .unwrap();
    // Second entry should still be findable
    let r = native_hashmap_get(
        &[Slot::Reference(Some(hm_ref)), Slot::Reference(Some(k2))],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    if let Slot::Reference(Some(found)) = r {
        assert_eq!(
            heap.get(found).unwrap().string_value,
            heap.get(v2).unwrap().string_value
        );
    } else {
        panic!("expected reference after remove");
    }
}

#[test]
fn native_hashmap_get_or_default_returns_second_entry_value() {
    let (mut heap, hm_ref, _k1, _v1, k2, v2) = make_hashmap_with_two_string_entries();
    let def = heap.allocate_string("default".to_string());
    let mut out: Vec<u8> = Vec::new();
    let result = native_hashmap_get_or_default(
        &[
            Slot::Reference(Some(hm_ref)),
            Slot::Reference(Some(k2)),
            Slot::Reference(Some(def)),
        ],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    if let Slot::Reference(Some(r)) = result {
        assert_ne!(
            heap.get(r).unwrap().string_value,
            heap.get(def).unwrap().string_value
        );
        assert_eq!(
            heap.get(r).unwrap().string_value,
            heap.get(v2).unwrap().string_value
        );
    } else {
        panic!("expected reference");
    }
}

#[test]
fn native_hashmap_get_or_default_returns_default_when_absent() {
    let (mut heap, hm_ref, _k1, _v1, _k2, _v2) = make_hashmap_with_two_string_entries();
    let missing_key = heap.allocate_string("missing".to_string());
    let def = heap.allocate_string("default_val".to_string());
    let mut out: Vec<u8> = Vec::new();
    let result = native_hashmap_get_or_default(
        &[
            Slot::Reference(Some(hm_ref)),
            Slot::Reference(Some(missing_key)),
            Slot::Reference(Some(def)),
        ],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    if let Slot::Reference(Some(r)) = result {
        assert_eq!(r, def);
    } else {
        panic!("expected reference");
    }
}

#[test]
fn native_hashmap_put_update_existing_key_returns_old_value() {
    let (mut heap, hm_ref, k1, v1, _k2, _v2) = make_hashmap_with_two_string_entries();
    let new_val = heap.allocate_string("new_val1".to_string());
    let mut out: Vec<u8> = Vec::new();
    // Update k1 → should return old value v1
    let old = native_hashmap_put(
        &[
            Slot::Reference(Some(hm_ref)),
            Slot::Reference(Some(k1)),
            Slot::Reference(Some(new_val)),
        ],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    if let Slot::Reference(Some(r)) = old {
        assert_eq!(
            heap.get(r).unwrap().string_value,
            heap.get(v1).unwrap().string_value
        );
    } else {
        panic!("expected old value reference, got {old:?}");
    }
}

// ===========================================================================
// execute_class() synthetic tests — covers execute_class opcode paths
// (Distinct from execute() tests; kills mutants in execute_class body.)
// ===========================================================================

/// Run an instruction stream directly through `execute_class()` using a
/// synthetic `ClassContext`, killing mutants in the `execute_class()` switch body.
#[allow(clippy::needless_pass_by_value)]
fn execute_class_synthetic(
    instructions: Vec<(usize, Instruction)>,
    args: Vec<Slot>,
    max_stack: u16,
    max_locals: u16,
    descriptor: &str,
) -> VmResult<Option<Slot>> {
    use std::sync::Arc;
    let pc_to_idx: std::collections::HashMap<usize, usize> = instructions
        .iter()
        .enumerate()
        .map(|(idx, (pc, _))| (*pc, idx))
        .collect();
    let method = MethodEntry {
        name: "syntest".to_string(),
        descriptor: descriptor.to_string(),
        is_public: true,
        is_static: true,
        instructions: instructions.into(),
        max_stack,
        max_locals,
        exception_table: vec![],
        pc_to_idx: Arc::new(pc_to_idx),
    };
    let ctx = ClassContext {
        class_name: "SynTest".to_string(),
        super_class: None,
        interfaces: vec![],
        constant_pool: vec![None],
        methods: vec![method],
        fields: vec![],
        static_fields: vec![],
        instance_field_count: 0,
        bootstrap_methods: vec![],
    };
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = make_simple_loader();
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "SynTest",
        "syntest",
        descriptor,
        &args,
    )
}

// ---- execute_class: Iushr ----

#[test]
fn ec_iushr_masks_shift_count() {
    // -8 >>> 2: mask 2 & 0x1F = 2; if | instead: 2|31=31 → result differs
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::Iushr),
            (3, Instruction::Ireturn),
        ],
        vec![Slot::Int(-8), Slot::Int(2)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    // (-8u32) >> 2 = 0x3FFFFFFE = 1073741822
    assert_eq!(r, Slot::Int(1_073_741_822));
}

// ---- execute_class: Iand / Ior / Ixor ----

#[test]
fn ec_iand_selects_common_bits() {
    // 0b11110000 & 0b10101010 = 0b10100000 = 160
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::Iand),
            (3, Instruction::Ireturn),
        ],
        vec![Slot::Int(0b1111_0000), Slot::Int(0b1010_1010)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0b1010_0000)); // 160
}

#[test]
fn ec_ior_combines_bits() {
    // 12 | 10 = 14
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::Ior),
            (3, Instruction::Ireturn),
        ],
        vec![Slot::Int(12), Slot::Int(10)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(14));
}

#[test]
fn ec_ixor_flips_differing_bits() {
    // 5 ^ 3 = 6; if | then 7; if & then 1
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::Ixor),
            (3, Instruction::Ireturn),
        ],
        vec![Slot::Int(5), Slot::Int(3)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(6));
}

// ---- execute_class: Ldiv / Lrem division-by-zero checks ----

#[test]
fn ec_ldiv_normal_returns_quotient() {
    // 10L / 3L = 3L; if b==0 check becomes !=, divides when b=3 (not 0) → errors instead
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Lload0),
            (1, Instruction::Lload1),
            (2, Instruction::Ldiv),
            (3, Instruction::Lreturn),
        ],
        vec![Slot::Long(10), Slot::Long(3)],
        4,
        2,
        "()J",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(3));
}

#[test]
fn ec_lrem_normal_returns_remainder() {
    // 10L % 3L = 1L; same kill logic as ldiv
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Lload0),
            (1, Instruction::Lload1),
            (2, Instruction::Lrem),
            (3, Instruction::Lreturn),
        ],
        vec![Slot::Long(10), Slot::Long(3)],
        4,
        2,
        "()J",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(1));
}

// ---- execute_class: Lshl / Lshr / Lushr masking ----

#[test]
fn ec_lshl_masks_shift_by_63_not_127() {
    // s=65, 65 & 0x3F = 1; if | then 65|63=127 → wrapping_shl(127) ≠ wrapping_shl(1)
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Lload0),
            (1, Instruction::Iload1),
            (2, Instruction::Lshl),
            (3, Instruction::Lreturn),
        ],
        vec![Slot::Long(1), Slot::Int(65)],
        4,
        2,
        "()J",
    )
    .unwrap()
    .unwrap();
    // 1L << 1 = 2
    assert_eq!(r, Slot::Long(2));
}

#[test]
fn ec_lshr_masks_shift_count() {
    // -8L >> 65; 65 & 63 = 1 → -8 >> 1 = -4; if | then 65|63=127 → 127%64=63 → -8>>63=-1
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Lload0),
            (1, Instruction::Iload1),
            (2, Instruction::Lshr),
            (3, Instruction::Lreturn),
        ],
        vec![Slot::Long(-8), Slot::Int(65)],
        4,
        2,
        "()J",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(-4));
}

#[test]
fn ec_lushr_right_shift_direction() {
    // i64::MIN >>> 1 = large positive; if << instead: 0
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Lload0),
            (1, Instruction::Iload1),
            (2, Instruction::Lushr),
            (3, Instruction::Lreturn),
        ],
        vec![Slot::Long(i64::MIN), Slot::Int(1)],
        4,
        2,
        "()J",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(4_611_686_018_427_387_904));
}

#[test]
fn ec_lushr_masks_shift_count() {
    // i64::MIN >>> 65; 65 & 63 = 1 → same as >>>1; if | then 65|63=127 → 127%64=63 → 1
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Lload0),
            (1, Instruction::Iload1),
            (2, Instruction::Lushr),
            (3, Instruction::Lreturn),
        ],
        vec![Slot::Long(i64::MIN), Slot::Int(65)],
        4,
        2,
        "()J",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(4_611_686_018_427_387_904));
}

// ---- execute_class: Land / Lor / Lxor ----

#[test]
fn ec_land_selects_common_bits() {
    // 0xF0F0F0F0L & 0x0F0F0F0FL = 0 (no common bits)
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Lload0),
            (1, Instruction::Lload1),
            (2, Instruction::Land),
            (3, Instruction::Lreturn),
        ],
        vec![Slot::Long(0xF0F0_F0F0), Slot::Long(0x0F0F_0F0F)],
        4,
        2,
        "()J",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(0));
}

#[test]
fn ec_lor_combines_bits() {
    // 0b1100L | 0b0110L = 0b1110 = 14
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Lload0),
            (1, Instruction::Lload1),
            (2, Instruction::Lor),
            (3, Instruction::Lreturn),
        ],
        vec![Slot::Long(0b1100), Slot::Long(0b0110)],
        4,
        2,
        "()J",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(14));
}

#[test]
fn ec_lxor_flips_differing_bits() {
    // 0b1010L ^ 0b1010L = 0; | or & both give 0b1010=10
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Lload0),
            (1, Instruction::Lload1),
            (2, Instruction::Lxor),
            (3, Instruction::Lreturn),
        ],
        vec![Slot::Long(0b1010), Slot::Long(0b1010)],
        4,
        2,
        "()J",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(0));
}

// ---- execute_class: Lcmp (-1 arm) ----

#[test]
fn ec_lcmp_less_than_returns_minus_one() {
    // lcmp(2, 5) → -1; delete - mutant returns 1
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Lload0),
            (1, Instruction::Lload1),
            (2, Instruction::Lcmp),
            (3, Instruction::Ireturn),
        ],
        vec![Slot::Long(2), Slot::Long(5)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(-1));
}

#[test]
fn ec_lcmp_equal_returns_zero() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Lload0),
            (1, Instruction::Lload1),
            (2, Instruction::Lcmp),
            (3, Instruction::Ireturn),
        ],
        vec![Slot::Long(7), Slot::Long(7)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn ec_lcmp_greater_than_returns_one() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Lload0),
            (1, Instruction::Lload1),
            (2, Instruction::Lcmp),
            (3, Instruction::Ireturn),
        ],
        vec![Slot::Long(10), Slot::Long(3)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

// ---- execute_class: Float arithmetic ----

#[test]
fn ec_fadd_adds_floats() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Fadd),
            (3, Instruction::Freturn),
        ],
        vec![Slot::Float(2.0), Slot::Float(3.0)],
        4,
        2,
        "()F",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Float(5.0));
}

#[test]
fn ec_fsub_subtracts_floats() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Fsub),
            (3, Instruction::Freturn),
        ],
        vec![Slot::Float(7.0), Slot::Float(3.0)],
        4,
        2,
        "()F",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Float(4.0));
}

#[test]
fn ec_fmul_multiplies_floats() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Fmul),
            (3, Instruction::Freturn),
        ],
        vec![Slot::Float(4.0), Slot::Float(3.0)],
        4,
        2,
        "()F",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Float(12.0));
}

#[test]
fn ec_fdiv_divides_floats() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Fdiv),
            (3, Instruction::Freturn),
        ],
        vec![Slot::Float(10.0), Slot::Float(4.0)],
        4,
        2,
        "()F",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Float(2.5));
}

#[test]
fn ec_frem_float_remainder() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Frem),
            (3, Instruction::Freturn),
        ],
        vec![Slot::Float(10.0), Slot::Float(3.0)],
        4,
        2,
        "()F",
    )
    .unwrap()
    .unwrap();
    if let Slot::Float(v) = r {
        assert!((v - 1.0_f32).abs() < 0.001);
    } else {
        panic!("{r:?}");
    }
}

#[test]
fn ec_fneg_negates_float() {
    // -(-5.0) = 5.0; if delete - mutant: -5.0 != 5.0
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Fload0),
            (1, Instruction::Fneg),
            (2, Instruction::Freturn),
        ],
        vec![Slot::Float(-5.0)],
        4,
        1,
        "()F",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Float(5.0));
}

// ---- execute_class: Fcmpl / Fcmpg ----

#[test]
fn ec_fcmpl_greater_returns_1() {
    // a > b → 1; if > mutated to < then returns -1
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Fcmpl),
            (3, Instruction::Ireturn),
        ],
        vec![Slot::Float(3.0), Slot::Float(2.0)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn ec_fcmpl_less_returns_minus_1() {
    // a < b → -1; delete - mutant returns 1
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Fcmpl),
            (3, Instruction::Ireturn),
        ],
        vec![Slot::Float(2.0), Slot::Float(3.0)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(-1));
}

#[test]
fn ec_fcmpl_nan_returns_minus_1() {
    // NaN case for Fcmpl → -1; delete - mutant at 6033 returns 1
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Fcmpl),
            (3, Instruction::Ireturn),
        ],
        vec![Slot::Float(f32::NAN), Slot::Float(0.0)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(-1));
}

#[test]
fn ec_fcmpg_nan_returns_1() {
    // NaN case for Fcmpg → +1; verifies Fcmpg differs from Fcmpl for NaN
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Fload0),
            (1, Instruction::Fload1),
            (2, Instruction::Fcmpg),
            (3, Instruction::Ireturn),
        ],
        vec![Slot::Float(f32::NAN), Slot::Float(0.0)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

// ---- execute_class: Double arithmetic ----

#[test]
fn ec_dadd_adds_doubles() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Dadd),
            (3, Instruction::Dreturn),
        ],
        vec![Slot::Double(2.0), Slot::Double(3.0)],
        4,
        2,
        "()D",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Double(5.0));
}

#[test]
fn ec_dsub_subtracts_doubles() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Dsub),
            (3, Instruction::Dreturn),
        ],
        vec![Slot::Double(7.0), Slot::Double(3.0)],
        4,
        2,
        "()D",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Double(4.0));
}

#[test]
fn ec_dmul_multiplies_doubles() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Dmul),
            (3, Instruction::Dreturn),
        ],
        vec![Slot::Double(4.0), Slot::Double(3.0)],
        4,
        2,
        "()D",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Double(12.0));
}

#[test]
fn ec_ddiv_divides_doubles() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Ddiv),
            (3, Instruction::Dreturn),
        ],
        vec![Slot::Double(10.0), Slot::Double(4.0)],
        4,
        2,
        "()D",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Double(2.5));
}

#[test]
fn ec_drem_double_remainder() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Drem),
            (3, Instruction::Dreturn),
        ],
        vec![Slot::Double(10.0), Slot::Double(3.0)],
        4,
        2,
        "()D",
    )
    .unwrap()
    .unwrap();
    if let Slot::Double(v) = r {
        assert!((v - 1.0_f64).abs() < 1e-9);
    } else {
        panic!("{r:?}");
    }
}

#[test]
fn ec_dneg_negates_double() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Dload0),
            (1, Instruction::Dneg),
            (2, Instruction::Dreturn),
        ],
        vec![Slot::Double(-5.0)],
        4,
        1,
        "()D",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Double(5.0));
}

// ---- execute_class: Dcmpl / Dcmpg ----

#[test]
fn ec_dcmpl_greater_returns_1() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Dcmpl),
            (3, Instruction::Ireturn),
        ],
        vec![Slot::Double(3.0), Slot::Double(2.0)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn ec_dcmpl_less_returns_minus_1() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Dcmpl),
            (3, Instruction::Ireturn),
        ],
        vec![Slot::Double(2.0), Slot::Double(3.0)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(-1));
}

#[test]
fn ec_dcmpl_nan_returns_minus_1() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Dcmpl),
            (3, Instruction::Ireturn),
        ],
        vec![Slot::Double(f64::NAN), Slot::Double(0.0)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(-1));
}

#[test]
fn ec_dcmpg_nan_returns_1() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Dload0),
            (1, Instruction::Dload1),
            (2, Instruction::Dcmpg),
            (3, Instruction::Ireturn),
        ],
        vec![Slot::Double(f64::NAN), Slot::Double(0.0)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

// ---- execute_class: Conditional branches ----
// Branch layout: (0,push), (1,Ifxx(5))→target=6, (4,Iconst0)→not-taken, (5,Ireturn),
//                (6,Iconst1)→taken, (7,Ireturn)

#[test]
fn ec_iflt_taken_on_negative() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iflt(5)), // target = 1+5 = 6
            (4, Instruction::Iconst0),
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst1),
            (7, Instruction::Ireturn),
        ],
        vec![Slot::Int(-1)],
        4,
        1,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn ec_iflt_not_taken_on_zero() {
    // zero is not < 0; mutant (< → ==) makes 0==0 → taken → 1 ≠ 0
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iflt(5)),
            (4, Instruction::Iconst0),
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst1),
            (7, Instruction::Ireturn),
        ],
        vec![Slot::Int(0)],
        4,
        1,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn ec_iflt_not_taken_on_positive() {
    // +1 not < 0; mutant (< → >) makes 1>0 → taken → 1 ≠ 0
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iflt(5)),
            (4, Instruction::Iconst0),
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst1),
            (7, Instruction::Ireturn),
        ],
        vec![Slot::Int(1)],
        4,
        1,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn ec_ifgt_taken_on_positive() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iload0),
            (1, Instruction::Ifgt(5)), // target = 6
            (4, Instruction::Iconst0),
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst1),
            (7, Instruction::Ireturn),
        ],
        vec![Slot::Int(1)],
        4,
        1,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn ec_ifgt_not_taken_on_zero() {
    // 0 not > 0; mutant (> → ==): 0==0 → taken → 1 ≠ 0
    // also kills > → >= mutant
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iload0),
            (1, Instruction::Ifgt(5)),
            (4, Instruction::Iconst0),
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst1),
            (7, Instruction::Ireturn),
        ],
        vec![Slot::Int(0)],
        4,
        1,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn ec_ifgt_not_taken_on_negative() {
    // -1 not > 0; mutant (> → <): -1<0 → taken → 1 ≠ 0
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iload0),
            (1, Instruction::Ifgt(5)),
            (4, Instruction::Iconst0),
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst1),
            (7, Instruction::Ireturn),
        ],
        vec![Slot::Int(-1)],
        4,
        1,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn ec_ificmpeq_taken_when_equal() {
    // a==b → taken; mutant (== → !=): not taken → 0 ≠ 1
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::IfIcmpeq(5)), // target = 2+5 = 7
            (5, Instruction::Iconst0),
            (6, Instruction::Ireturn),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ],
        vec![Slot::Int(3), Slot::Int(3)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn ec_ificmpeq_not_taken_when_unequal() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::IfIcmpeq(5)),
            (5, Instruction::Iconst0),
            (6, Instruction::Ireturn),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ],
        vec![Slot::Int(3), Slot::Int(4)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn ec_ificmplt_taken_when_less() {
    // 2 < 5 → taken; mutant (< → ==): 2==5 = false → 0 ≠ 1
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::IfIcmplt(5)),
            (5, Instruction::Iconst0),
            (6, Instruction::Ireturn),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ],
        vec![Slot::Int(2), Slot::Int(5)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn ec_ificmplt_not_taken_when_equal() {
    // 3 not < 3 → 0; mutant (< → ==): 3==3=true → 1; mutant (< → <=): 3<=3=true → 1
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::IfIcmplt(5)),
            (5, Instruction::Iconst0),
            (6, Instruction::Ireturn),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ],
        vec![Slot::Int(3), Slot::Int(3)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn ec_ificmplt_not_taken_when_greater() {
    // 5 not < 2 → 0; mutant (< → >): 5>2=true → 1
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::IfIcmplt(5)),
            (5, Instruction::Iconst0),
            (6, Instruction::Ireturn),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ],
        vec![Slot::Int(5), Slot::Int(2)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn ec_ificmple_taken_when_equal() {
    // 3 <= 3 → taken → 1; mutant (<= → >): 3>3=false → 0
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::IfIcmple(5)),
            (5, Instruction::Iconst0),
            (6, Instruction::Ireturn),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ],
        vec![Slot::Int(3), Slot::Int(3)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn ec_ificmple_not_taken_when_greater() {
    // 4 not <= 3 → 0; mutant (<= → >): 4>3=true → 1
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::IfIcmple(5)),
            (5, Instruction::Iconst0),
            (6, Instruction::Ireturn),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ],
        vec![Slot::Int(4), Slot::Int(3)],
        4,
        2,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

// ---- native argument extraction helper coverage ----

#[test]
fn test_extract_ref_arg_success() {
    let args = vec![Slot::Reference(Some(42))];
    assert_eq!(extract_ref_arg(&args, 0).unwrap(), 42);
}

#[test]
fn test_extract_ref_arg_null() {
    let args = vec![Slot::Reference(None)];
    assert!(matches!(
        extract_ref_arg(&args, 0).unwrap_err(),
        VmError::NullPointerException
    ));
}

#[test]
fn test_extract_ref_arg_type_mismatch() {
    let args = vec![Slot::Int(1)];
    assert!(matches!(
        extract_ref_arg(&args, 0).unwrap_err(),
        VmError::NullPointerException
    ));
}

#[test]
fn test_extract_int_arg_success() {
    let args = vec![Slot::Int(42)];
    assert_eq!(extract_int_arg(&args, 0).unwrap(), 42);
}

#[test]
fn test_extract_int_arg_type_mismatch() {
    let args = vec![Slot::Reference(Some(1))];
    assert!(matches!(
        extract_int_arg(&args, 0).unwrap_err(),
        VmError::TypeMismatch {
            expected: "Int",
            got: "other"
        }
    ));
}

#[test]
fn test_extract_long_arg_missing() {
    let args = vec![];
    assert!(matches!(
        extract_long_arg(&args, 0).unwrap_err(),
        VmError::TypeMismatch {
            expected: "Long",
            got: "other"
        }
    ));
}

#[test]
fn test_extract_long_arg_success() {
    let args = vec![Slot::Long(42)];
    assert_eq!(extract_long_arg(&args, 0).unwrap(), 42);
}

#[test]
fn test_extract_long_arg_type_mismatch() {
    let args = vec![Slot::Reference(Some(1))];
    assert!(matches!(
        extract_long_arg(&args, 0).unwrap_err(),
        VmError::TypeMismatch {
            expected: "Long",
            got: "other"
        }
    ));
}

#[test]
fn test_extract_float_arg_missing() {
    let args = vec![];
    assert!(matches!(
        extract_float_arg(&args, 0).unwrap_err(),
        VmError::TypeMismatch {
            expected: "Float",
            got: "other"
        }
    ));
}

#[test]
fn test_extract_float_arg_success() {
    let args = vec![Slot::Float(42.0)];
    assert!((extract_float_arg(&args, 0).unwrap() - 42.0).abs() < f32::EPSILON);
}

#[test]
fn test_extract_float_arg_type_mismatch() {
    let args = vec![Slot::Reference(Some(1))];
    assert!(matches!(
        extract_float_arg(&args, 0).unwrap_err(),
        VmError::TypeMismatch {
            expected: "Float",
            got: "other"
        }
    ));
}

#[test]
fn test_extract_double_arg_missing() {
    let args = vec![];
    assert!(matches!(
        extract_double_arg(&args, 0).unwrap_err(),
        VmError::TypeMismatch {
            expected: "Double",
            got: "other"
        }
    ));
}

#[test]
fn test_extract_double_arg_success() {
    let args = vec![Slot::Double(42.0)];
    assert!((extract_double_arg(&args, 0).unwrap() - 42.0).abs() < f64::EPSILON);
}

#[test]
fn test_extract_double_arg_type_mismatch() {
    let args = vec![Slot::Reference(Some(1))];
    assert!(matches!(
        extract_double_arg(&args, 0).unwrap_err(),
        VmError::TypeMismatch {
            expected: "Double",
            got: "other"
        }
    ));
}

#[test]
fn test_extract_io_fd_success() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Object".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(42);
    assert_eq!(extract_io_fd(&heap, r).unwrap(), 42);
}

#[test]
fn test_extract_io_fd_type_mismatch() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Object".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Reference(Some(1));
    assert!(matches!(
        extract_io_fd(&heap, r).unwrap_err(),
        VmError::JavaException { .. }
    ));
}

#[test]
fn test_extract_io_fd_at_success() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Object".to_string(), 2);
    heap.get_mut(r).unwrap().fields[1] = Slot::Int(42);
    assert_eq!(extract_io_fd_at(&heap, r, 1).unwrap(), 42);
}

#[test]
fn test_extract_io_fd_at_type_mismatch() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Object".to_string(), 2);
    heap.get_mut(r).unwrap().fields[1] = Slot::Reference(Some(1));
    assert!(matches!(
        extract_io_fd_at(&heap, r, 1).unwrap_err(),
        VmError::JavaException { .. }
    ));
}

// ---- execute_class: Newarray init for Long/Float/Double ----

#[test]
fn ec_newarray_long_default_slot_is_long_zero() {
    // Create long[1]; arm deleted → fields stay Int(0) → Laload TypeMismatch
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iconst1),
            (
                1,
                Instruction::Newarray(duke_bytecode::instruction::ArrayType::Long),
            ),
            (3, Instruction::Astore0),
            (4, Instruction::Aload0),
            (5, Instruction::Iconst0),
            (6, Instruction::Laload),
            (7, Instruction::Lreturn),
        ],
        vec![],
        4,
        1,
        "()J",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(0));
}

#[test]
fn ec_newarray_float_default_slot_is_float_zero() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iconst1),
            (
                1,
                Instruction::Newarray(duke_bytecode::instruction::ArrayType::Float),
            ),
            (3, Instruction::Astore0),
            (4, Instruction::Aload0),
            (5, Instruction::Iconst0),
            (6, Instruction::Faload),
            (7, Instruction::Freturn),
        ],
        vec![],
        4,
        1,
        "()F",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Float(0.0));
}

#[test]
fn ec_newarray_double_default_slot_is_double_zero() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iconst1),
            (
                1,
                Instruction::Newarray(duke_bytecode::instruction::ArrayType::Double),
            ),
            (3, Instruction::Astore0),
            (4, Instruction::Aload0),
            (5, Instruction::Iconst0),
            (6, Instruction::Daload),
            (7, Instruction::Dreturn),
        ],
        vec![],
        4,
        1,
        "()D",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Double(0.0));
}

// ---- execute_class: Array bounds — kills || → && and < mutations ----

/// Create int[3], access valid idx=0 → succeeds.
/// Kills `idx_val < 0` mutated to `== 0` and `<= 0` (idx=0 would false-trigger).
#[test]
fn ec_iaload_valid_idx0_succeeds() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (
                2,
                Instruction::Newarray(duke_bytecode::instruction::ArrayType::Int),
            ),
            (4, Instruction::Astore0),
            (5, Instruction::Aload0),
            (6, Instruction::Iconst0),
            (7, Instruction::Iaload),
            (8, Instruction::Ireturn),
        ],
        vec![],
        4,
        1,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

/// Access valid idx=1 of int[3]; kills `< → >` (1>0=true would wrongly error).
#[test]
fn ec_iaload_valid_idx1_succeeds() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (
                2,
                Instruction::Newarray(duke_bytecode::instruction::ArrayType::Int),
            ),
            (4, Instruction::Astore0),
            (5, Instruction::Aload0),
            (6, Instruction::Iconst1),
            (7, Instruction::Iaload),
            (8, Instruction::Ireturn),
        ],
        vec![],
        4,
        1,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

/// Access OOB idx=3 of int[3]; kills `|| → &&` (false && true = no error).
#[test]
fn ec_iaload_oob_at_length_errors() {
    let err = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (
                2,
                Instruction::Newarray(duke_bytecode::instruction::ArrayType::Int),
            ),
            (4, Instruction::Astore0),
            (5, Instruction::Aload0),
            (6, Instruction::Bipush(3)), // idx = length = OOB
            (8, Instruction::Iaload),
            (9, Instruction::Ireturn),
        ],
        vec![],
        4,
        1,
        "()I",
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

/// Iastore OOB at length; kills its || → && mutant.
#[test]
fn ec_iastore_oob_at_length_errors() {
    let err = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (
                2,
                Instruction::Newarray(duke_bytecode::instruction::ArrayType::Int),
            ),
            (4, Instruction::Astore0),
            (5, Instruction::Aload0),
            (6, Instruction::Bipush(3)), // idx=3 = OOB
            (8, Instruction::Iconst5),   // value
            (9, Instruction::Iastore),
            (10, Instruction::Iconst0),
            (11, Instruction::Ireturn),
        ],
        vec![],
        4,
        1,
        "()I",
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

/// Laload: valid idx=0 succeeds (kills < → == at Laload bounds).
#[test]
fn ec_laload_valid_idx0_succeeds() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (
                2,
                Instruction::Newarray(duke_bytecode::instruction::ArrayType::Long),
            ),
            (4, Instruction::Astore0),
            (5, Instruction::Aload0),
            (6, Instruction::Iconst0),
            (7, Instruction::Laload),
            (8, Instruction::Lreturn),
        ],
        vec![],
        4,
        1,
        "()J",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(0));
}

/// Laload: valid idx=1 succeeds (kills < → > at Laload bounds).
#[test]
fn ec_laload_valid_idx1_succeeds() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (
                2,
                Instruction::Newarray(duke_bytecode::instruction::ArrayType::Long),
            ),
            (4, Instruction::Astore0),
            (5, Instruction::Aload0),
            (6, Instruction::Iconst1),
            (7, Instruction::Laload),
            (8, Instruction::Lreturn),
        ],
        vec![],
        4,
        1,
        "()J",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(0));
}

/// Laload OOB kills || → &&.
#[test]
fn ec_laload_oob_at_length_errors() {
    let err = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (
                2,
                Instruction::Newarray(duke_bytecode::instruction::ArrayType::Long),
            ),
            (4, Instruction::Astore0),
            (5, Instruction::Aload0),
            (6, Instruction::Bipush(3)),
            (8, Instruction::Laload),
            (9, Instruction::Lreturn),
        ],
        vec![],
        4,
        1,
        "()J",
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

/// Lastore OOB kills || → &&.
#[test]
fn ec_lastore_oob_at_length_errors() {
    let err = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (
                2,
                Instruction::Newarray(duke_bytecode::instruction::ArrayType::Long),
            ),
            (4, Instruction::Astore0),
            (5, Instruction::Aload0),
            (6, Instruction::Bipush(3)), // idx=3 OOB
            (8, Instruction::Lconst0),
            (9, Instruction::Lastore),
            (10, Instruction::Iconst0),
            (11, Instruction::Ireturn),
        ],
        vec![],
        4,
        1,
        "()I",
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

/// Faload valid idx=0 and idx=1.
#[test]
fn ec_faload_valid_idx0_succeeds() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (
                2,
                Instruction::Newarray(duke_bytecode::instruction::ArrayType::Float),
            ),
            (4, Instruction::Astore0),
            (5, Instruction::Aload0),
            (6, Instruction::Iconst0),
            (7, Instruction::Faload),
            (8, Instruction::Freturn),
        ],
        vec![],
        4,
        1,
        "()F",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Float(0.0));
}

#[test]
fn ec_faload_oob_at_length_errors() {
    let err = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (
                2,
                Instruction::Newarray(duke_bytecode::instruction::ArrayType::Float),
            ),
            (4, Instruction::Astore0),
            (5, Instruction::Aload0),
            (6, Instruction::Bipush(3)),
            (8, Instruction::Faload),
            (9, Instruction::Freturn),
        ],
        vec![],
        4,
        1,
        "()F",
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

#[test]
fn ec_fastore_oob_at_length_errors() {
    let err = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (
                2,
                Instruction::Newarray(duke_bytecode::instruction::ArrayType::Float),
            ),
            (4, Instruction::Astore0),
            (5, Instruction::Aload0),
            (6, Instruction::Bipush(3)), // idx=3 OOB
            (8, Instruction::Fconst0),
            (9, Instruction::Fastore),
            (10, Instruction::Iconst0),
            (11, Instruction::Ireturn),
        ],
        vec![],
        4,
        1,
        "()I",
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

#[test]
fn ec_daload_valid_idx0_succeeds() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (
                2,
                Instruction::Newarray(duke_bytecode::instruction::ArrayType::Double),
            ),
            (4, Instruction::Astore0),
            (5, Instruction::Aload0),
            (6, Instruction::Iconst0),
            (7, Instruction::Daload),
            (8, Instruction::Dreturn),
        ],
        vec![],
        4,
        1,
        "()D",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Double(0.0));
}

#[test]
fn ec_daload_valid_idx1_succeeds() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (
                2,
                Instruction::Newarray(duke_bytecode::instruction::ArrayType::Double),
            ),
            (4, Instruction::Astore0),
            (5, Instruction::Aload0),
            (6, Instruction::Iconst1),
            (7, Instruction::Daload),
            (8, Instruction::Dreturn),
        ],
        vec![],
        4,
        1,
        "()D",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Double(0.0));
}

#[test]
fn ec_daload_oob_at_length_errors() {
    let err = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (
                2,
                Instruction::Newarray(duke_bytecode::instruction::ArrayType::Double),
            ),
            (4, Instruction::Astore0),
            (5, Instruction::Aload0),
            (6, Instruction::Bipush(3)),
            (8, Instruction::Daload),
            (9, Instruction::Dreturn),
        ],
        vec![],
        4,
        1,
        "()D",
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

#[test]
fn ec_dastore_oob_at_length_errors() {
    let err = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (
                2,
                Instruction::Newarray(duke_bytecode::instruction::ArrayType::Double),
            ),
            (4, Instruction::Astore0),
            (5, Instruction::Aload0),
            (6, Instruction::Bipush(3)), // idx=3 OOB
            (8, Instruction::Dconst0),
            (9, Instruction::Dastore),
            (10, Instruction::Iconst0),
            (11, Instruction::Ireturn),
        ],
        vec![],
        4,
        1,
        "()I",
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- run_class_int on Arithmetic.class via execute_class() ----
// These tests exercise the same operations as run_static_int but via execute_class,
// killing mutants in the execute_class() opcode body.

#[test]
fn ec_arithmetic_bitwise_xor() {
    // 5 ^ 3 = 6; via execute_class() kills Ixor mutants in execute_class
    assert_eq!(
        run_class_int("Arithmetic.class", "bitwiseXor", "(II)I", vec![5, 3]),
        6
    );
}

#[test]
fn ec_arithmetic_bitwise_and() {
    assert_eq!(
        run_class_int(
            "Arithmetic.class",
            "bitwiseAnd",
            "(II)I",
            vec![0b1111, 0b1010]
        ),
        0b1010
    );
}

#[test]
fn ec_arithmetic_bitwise_or() {
    assert_eq!(
        run_class_int(
            "Arithmetic.class",
            "bitwiseOr",
            "(II)I",
            vec![0b1111, 0b1010]
        ),
        0b1111
    );
}

#[test]
fn ec_arithmetic_abs_negative() {
    // abs(-5) = 5; exercises conditional branch in execute_class
    assert_eq!(
        run_class_int("Arithmetic.class", "abs", "(I)I", vec![-5]),
        5
    );
}

#[test]
fn ec_arithmetic_abs_positive() {
    assert_eq!(run_class_int("Arithmetic.class", "abs", "(I)I", vec![5]), 5);
}

#[test]
fn ec_arithmetic_max_first_greater() {
    assert_eq!(
        run_class_int("Arithmetic.class", "max", "(II)I", vec![7, 3]),
        7
    );
}

#[test]
fn ec_arithmetic_max_second_greater() {
    assert_eq!(
        run_class_int("Arithmetic.class", "max", "(II)I", vec![3, 7]),
        7
    );
}

#[test]
fn ec_arithmetic_clamp_in_range() {
    assert_eq!(
        run_class_int("Arithmetic.class", "clamp", "(III)I", vec![5, 1, 10]),
        5
    );
}

#[test]
fn ec_arithmetic_clamp_below_lo() {
    assert_eq!(
        run_class_int("Arithmetic.class", "clamp", "(III)I", vec![0, 1, 10]),
        1
    );
}

#[test]
fn ec_arithmetic_clamp_above_hi() {
    assert_eq!(
        run_class_int("Arithmetic.class", "clamp", "(III)I", vec![15, 1, 10]),
        10
    );
}

#[test]
fn ec_arithmetic_fibonacci_10() {
    // Exercises loop with IfIcmple / sum additions in execute_class
    assert_eq!(
        run_class_int("Arithmetic.class", "fibonacci", "(I)I", vec![10]),
        55
    );
}

#[test]
fn ec_arithmetic_sum_to_100() {
    // Exercises += accumulation loop in execute_class
    assert_eq!(
        run_class_int("Arithmetic.class", "sumTo", "(I)I", vec![100]),
        5050
    );
}

#[test]
fn ec_arithmetic_factorial_5() {
    assert_eq!(
        run_class_int("Arithmetic.class", "factorial", "(I)I", vec![5]),
        120
    );
}

// =========================================================================
// Fifth batch: execute() opcode mutant kills
// =========================================================================

// ---- execute(): Ldiv / Lrem zero check (== → !=) ----

#[test]
fn execute_ldiv_by_zero_errors() {
    // Mutant: replace == with != → division proceeds instead of erroring
    let instructions = vec![
        (0, Instruction::Lconst1),
        (1, Instruction::Lconst0),
        (2, Instruction::Ldiv),
        (3, Instruction::Lreturn),
    ];
    let err = execute(&instructions, &[], vec![], 4, 1).unwrap_err();
    assert!(matches!(err, VmError::DivisionByZero));
}

#[test]
fn execute_lrem_by_zero_errors() {
    let instructions = vec![
        (0, Instruction::Lconst1),
        (1, Instruction::Lconst0),
        (2, Instruction::Lrem),
        (3, Instruction::Lreturn),
    ];
    let err = execute(&instructions, &[], vec![], 4, 1).unwrap_err();
    assert!(matches!(err, VmError::DivisionByZero));
}

// ---- execute(): Lor |→^ ----

#[test]
fn execute_lor_overlapping_bits_is_not_xor() {
    // a=0b11=3, b=0b10=2 → a|b=3, a^b=1; asserts OR result
    let instructions = vec![
        (0, Instruction::Lconst1), // push 1L
        (1, Instruction::Lconst1), // push 1L
        (2, Instruction::Ladd),    // 1+1=2L
        (3, Instruction::Lconst1), // push 1L
        (4, Instruction::Ladd),    // 2+1=3L  (this is a=3)
        (5, Instruction::Lconst1), // push 1L
        (6, Instruction::Lconst1), // push 1L
        (7, Instruction::Ladd),    // 1+1=2L  (this is b=2)
        (8, Instruction::Lor),     // 3|2=3, but 3^2=1
        (9, Instruction::Lreturn),
    ];
    let r = execute(&instructions, &[], vec![], 8, 1).unwrap().unwrap();
    assert_eq!(r, Slot::Long(3));
}

// ---- execute(): Lcmp delete - ----

#[test]
fn execute_lcmp_less_than_returns_minus_one() {
    // Mutant: delete - → returns 1 instead of -1
    let instructions = vec![
        (0, Instruction::Lconst0), // push 0L (a)
        (1, Instruction::Lconst1), // push 1L (b)
        (2, Instruction::Lcmp),    // 0 < 1 → -1
        (3, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &[], vec![], 4, 1).unwrap().unwrap();
    assert_eq!(r, Slot::Int(-1));
}

// ---- execute(): IfIcmpeq ==→!= / IfIcmpne !=→== ----

#[test]
fn execute_ificmpeq_taken_when_equal() {
    // a==b → jump to return 1; not taken → return 0
    // Mutant ==→!= causes: a==b → not taken → return 0 (fail)
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Iconst2),
        (2, Instruction::IfIcmpeq(5)), // if a==b jump to 2+5=7
        (5, Instruction::Iconst0),
        (6, Instruction::Ireturn),
        (7, Instruction::Iconst1),
        (8, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &[], vec![], 4, 1).unwrap().unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_ificmpeq_not_taken_when_unequal() {
    let instructions = vec![
        (0, Instruction::Iconst1),
        (1, Instruction::Iconst2),
        (2, Instruction::IfIcmpeq(5)), // 1!=2 → not taken
        (5, Instruction::Iconst0),
        (6, Instruction::Ireturn),
        (7, Instruction::Iconst1),
        (8, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &[], vec![], 4, 1).unwrap().unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn execute_ificmpne_taken_when_unequal() {
    // 1!=2 → taken → jump to PC 7 → Iconst1 → return 1
    // Mutant !=→== would check 1==2 (false) → not taken → Iconst0 → return 0
    let instructions = vec![
        (0, Instruction::Iconst1),
        (1, Instruction::Iconst2),
        (2, Instruction::IfIcmpne(5)), // 1!=2 → taken → 2+5=7
        (5, Instruction::Iconst0),
        (6, Instruction::Ireturn),
        (7, Instruction::Iconst1),
        (8, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &[], vec![], 4, 1).unwrap().unwrap();
    assert_eq!(r, Slot::Int(1)); // taken → Iconst1
}

#[test]
fn execute_ificmpne_jump_taken_when_unequal() {
    // 3!=5 → taken → jump to PC 7 → Iconst1 → 1
    let instructions = vec![
        (0, Instruction::Iconst3),
        (1, Instruction::Iconst5),
        (2, Instruction::IfIcmpne(5)), // 3!=5 → taken → 2+5=7
        (5, Instruction::Iconst0),
        (6, Instruction::Ireturn),
        (7, Instruction::Iconst1),
        (8, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &[], vec![], 4, 1).unwrap().unwrap();
    assert_eq!(r, Slot::Int(1)); // taken branch
}

#[test]
fn execute_ificmpne_not_taken_when_equal() {
    // a==b → !=  is false → not taken
    let instructions = vec![
        (0, Instruction::Iconst4),
        (1, Instruction::Iconst4),
        (2, Instruction::IfIcmpne(5)), // 4!=4 → false → not taken
        (5, Instruction::Iconst0),
        (6, Instruction::Ireturn),
        (7, Instruction::Iconst1),
        (8, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &[], vec![], 4, 1).unwrap().unwrap();
    assert_eq!(r, Slot::Int(0));
}

// ---- execute(): Newarray negative count → error ----

#[test]
fn execute_newarray_negative_count_errors() {
    // count=-1 → NegativeArraySize
    let instructions = vec![
        (0, Instruction::IconstM1),
        (1, Instruction::Newarray(ArrayType::Int)),
        (3, Instruction::Areturn),
    ];
    let err = execute(&instructions, &[], vec![], 2, 1).unwrap_err();
    assert!(matches!(err, VmError::NegativeArraySize { .. }));
}

#[test]
fn execute_newarray_zero_count_succeeds() {
    // count=0 is valid; mutant < → <= would reject count=0
    let instructions = vec![
        (0, Instruction::Iconst0),
        (1, Instruction::Newarray(ArrayType::Int)),
        (3, Instruction::Pop), // discard array ref
        (4, Instruction::Iconst1),
        (5, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &[], vec![], 4, 1).unwrap().unwrap();
    assert_eq!(r, Slot::Int(1));
}

// ---- execute(): Iaload / Iastore bounds ----

#[test]
fn execute_iaload_valid_idx0_returns_value() {
    // Kills < → == (if == then idx=0 would wrongly error)
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Int)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Iconst0),
        (5, Instruction::Bipush(77i8)),
        (6, Instruction::Iastore),
        (7, Instruction::Aload0),
        (8, Instruction::Iconst0),
        (9, Instruction::Iaload),
        (10, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &[], vec![], 4, 2).unwrap().unwrap();
    assert_eq!(r, Slot::Int(77));
}

#[test]
fn execute_iaload_valid_idx1_returns_value() {
    // Kills < → > (if > then idx=1 > 0 → true → wrongly errors)
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Int)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Iconst1),
        (5, Instruction::Bipush(55i8)),
        (6, Instruction::Iastore),
        (7, Instruction::Aload0),
        (8, Instruction::Iconst1),
        (9, Instruction::Iaload),
        (10, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &[], vec![], 4, 2).unwrap().unwrap();
    assert_eq!(r, Slot::Int(55));
}

#[test]
fn execute_iaload_oob_at_length_errors() {
    // idx=2, len=2: second cond true, first false → || gives error; && gives no error
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Int)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Bipush(2i8)),
        (5, Instruction::Iaload),
        (6, Instruction::Ireturn),
    ];
    let err = execute(&instructions, &[], vec![], 4, 2).unwrap_err();
    assert!(matches!(
        err,
        VmError::ArrayIndexOutOfBounds {
            index: 2,
            length: 2
        }
    ));
}

#[test]
fn execute_iastore_valid_idx0_stores() {
    // Kills < → == for Iastore
    let instructions = vec![
        (0, Instruction::Bipush(3i8)),
        (1, Instruction::Newarray(ArrayType::Int)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Iconst0),
        (5, Instruction::Bipush(99i8)),
        (6, Instruction::Iastore),
        (7, Instruction::Aload0),
        (8, Instruction::Iconst0),
        (9, Instruction::Iaload),
        (10, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &[], vec![], 4, 2).unwrap().unwrap();
    assert_eq!(r, Slot::Int(99));
}

#[test]
fn execute_iastore_valid_idx1_stores() {
    // Kills < → > for Iastore
    let instructions = vec![
        (0, Instruction::Bipush(3i8)),
        (1, Instruction::Newarray(ArrayType::Int)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Iconst1),
        (5, Instruction::Bipush(88i8)),
        (6, Instruction::Iastore),
        (7, Instruction::Aload0),
        (8, Instruction::Iconst1),
        (9, Instruction::Iaload),
        (10, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &[], vec![], 4, 2).unwrap().unwrap();
    assert_eq!(r, Slot::Int(88));
}

#[test]
fn execute_iastore_oob_at_length_errors() {
    // Kills || → && for Iastore
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Int)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Bipush(2i8)), // idx=2=len
        (5, Instruction::Iconst1),
        (6, Instruction::Iastore),
        (7, Instruction::Ireturn),
    ];
    let err = execute(&instructions, &[], vec![], 4, 2).unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- execute(): Laload / Lastore bounds ----

#[test]
fn execute_laload_valid_idx0_returns_value() {
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Long)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Iconst0),
        (5, Instruction::Lconst1),
        (6, Instruction::Lastore),
        (7, Instruction::Aload0),
        (8, Instruction::Iconst0),
        (9, Instruction::Laload),
        (10, Instruction::Lreturn),
    ];
    let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
    assert_eq!(r, Slot::Long(1));
}

#[test]
fn execute_laload_valid_idx1_returns_value() {
    // Kills < → > for Laload (valid at idx=1)
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Long)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Iconst1),
        (5, Instruction::Lconst1),
        (6, Instruction::Lastore),
        (7, Instruction::Aload0),
        (8, Instruction::Iconst1),
        (9, Instruction::Laload),
        (10, Instruction::Lreturn),
    ];
    let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
    assert_eq!(r, Slot::Long(1));
}

#[test]
fn execute_laload_oob_at_length_errors() {
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Long)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Bipush(2i8)),
        (5, Instruction::Laload),
        (6, Instruction::Lreturn),
    ];
    let err = execute(&instructions, &[], vec![], 6, 2).unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

#[test]
fn execute_lastore_valid_idx0_stores() {
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Long)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Iconst0),
        (5, Instruction::Lconst1),
        (6, Instruction::Lastore),
        (7, Instruction::Iconst1),
        (8, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_lastore_valid_idx1_stores() {
    // Kills < → > for Lastore
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Long)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Iconst1),
        (5, Instruction::Lconst1),
        (6, Instruction::Lastore),
        (7, Instruction::Iconst1),
        (8, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_lastore_oob_at_length_errors() {
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Long)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Bipush(2i8)),
        (5, Instruction::Lconst0),
        (6, Instruction::Lastore),
        (7, Instruction::Ireturn),
    ];
    let err = execute(&instructions, &[], vec![], 6, 2).unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- execute(): Faload / Fastore bounds ----

#[test]
fn execute_faload_valid_idx0_returns_value() {
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Float)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Iconst0),
        (5, Instruction::Fconst1),
        (6, Instruction::Fastore),
        (7, Instruction::Aload0),
        (8, Instruction::Iconst0),
        (9, Instruction::Faload),
        (10, Instruction::Freturn),
    ];
    let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
    assert_eq!(r, Slot::Float(1.0));
}

#[test]
fn execute_faload_valid_idx1_returns_value() {
    // Kills < → > for Faload
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Float)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Iconst1),
        (5, Instruction::Fconst1),
        (6, Instruction::Fastore),
        (7, Instruction::Aload0),
        (8, Instruction::Iconst1),
        (9, Instruction::Faload),
        (10, Instruction::Freturn),
    ];
    let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
    assert_eq!(r, Slot::Float(1.0));
}

#[test]
fn execute_faload_oob_at_length_errors() {
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Float)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Bipush(2i8)),
        (5, Instruction::Faload),
        (6, Instruction::Freturn),
    ];
    let err = execute(&instructions, &[], vec![], 6, 2).unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

#[test]
fn execute_fastore_valid_idx0_stores() {
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Float)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Iconst0),
        (5, Instruction::Fconst1),
        (6, Instruction::Fastore),
        (7, Instruction::Iconst1),
        (8, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_fastore_valid_idx1_stores() {
    // Kills < → > for Fastore
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Float)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Iconst1),
        (5, Instruction::Fconst1),
        (6, Instruction::Fastore),
        (7, Instruction::Iconst1),
        (8, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_fastore_oob_at_length_errors() {
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Float)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Bipush(2i8)),
        (5, Instruction::Fconst0),
        (6, Instruction::Fastore),
        (7, Instruction::Ireturn),
    ];
    let err = execute(&instructions, &[], vec![], 6, 2).unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- execute(): Daload / Dastore bounds ----

#[test]
fn execute_daload_valid_idx0_returns_value() {
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Double)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Iconst0),
        (5, Instruction::Dconst1),
        (6, Instruction::Dastore),
        (7, Instruction::Aload0),
        (8, Instruction::Iconst0),
        (9, Instruction::Daload),
        (10, Instruction::Dreturn),
    ];
    let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
    assert_eq!(r, Slot::Double(1.0));
}

#[test]
fn execute_daload_valid_idx1_returns_value() {
    // Kills < → > for Daload
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Double)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Iconst1),
        (5, Instruction::Dconst1),
        (6, Instruction::Dastore),
        (7, Instruction::Aload0),
        (8, Instruction::Iconst1),
        (9, Instruction::Daload),
        (10, Instruction::Dreturn),
    ];
    let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
    assert_eq!(r, Slot::Double(1.0));
}

#[test]
fn execute_daload_oob_at_length_errors() {
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Double)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Bipush(2i8)),
        (5, Instruction::Daload),
        (6, Instruction::Dreturn),
    ];
    let err = execute(&instructions, &[], vec![], 6, 2).unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

#[test]
fn execute_dastore_valid_idx0_stores() {
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Double)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Iconst0),
        (5, Instruction::Dconst1),
        (6, Instruction::Dastore),
        (7, Instruction::Iconst1),
        (8, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_dastore_valid_idx1_stores() {
    // Kills < → > for Dastore
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Double)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Iconst1),
        (5, Instruction::Dconst1),
        (6, Instruction::Dastore),
        (7, Instruction::Iconst1),
        (8, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_dastore_oob_at_length_errors() {
    let instructions = vec![
        (0, Instruction::Bipush(2i8)),
        (1, Instruction::Newarray(ArrayType::Double)),
        (2, Instruction::Astore0),
        (3, Instruction::Aload0),
        (4, Instruction::Bipush(2i8)),
        (5, Instruction::Dconst0),
        (6, Instruction::Dastore),
        (7, Instruction::Ireturn),
    ];
    let err = execute(&instructions, &[], vec![], 6, 2).unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- execute(): Tableswitch - → + (non-zero low) ----

#[test]
fn execute_tableswitch_nonzero_low_matches_key() {
    // key=2, low=1, offsets[key-low=1]=8 → jump to PC 2+8=10 → Bipush(99)
    // Mutant - → +: offsets[key+low=3] → OOB panic (offsets.len()=3)
    let instructions = vec![
        (0, Instruction::Iconst2),
        (
            2,
            Instruction::Tableswitch {
                default: 6i32, // 2+6=8 → Iconst0
                low: 1i32,
                high: 3i32,
                offsets: vec![6i32, 8i32, 6i32], // key=1→8, key=2→10, key=3→8
            },
        ),
        (8, Instruction::Iconst0),
        (9, Instruction::Ireturn),
        (10, Instruction::Bipush(99i8)),
        (12, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &[], vec![], 4, 1).unwrap().unwrap();
    assert_eq!(r, Slot::Int(99));
}

// ---- execute(): Checkcast null match arm deletion ----

#[test]
fn execute_checkcast_null_passes() {
    // Mutant: delete null arm → null falls to _ → TypeMismatch error
    use duke_classfile::types::CpIndex;
    let cp = vec![
        None,
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("foo".to_string())),
    ];
    let instructions = vec![
        (0, Instruction::AconstNull),
        (1, Instruction::Checkcast(CpIndex(1))),
        (4, Instruction::Iconst1),
        (5, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &cp, vec![], 4, 1).unwrap().unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_checkcast_matching_ref_passes() {
    // Kills == → != (matching class → pass; mutant rejects when class matches)
    use duke_classfile::types::CpIndex;
    let cp = vec![
        None,
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("[I".to_string())), // int array class
    ];
    // Create an int array (class_name="[I"), then checkcast to "[I" → should pass
    let instructions = vec![
        (0, Instruction::Bipush(1i8)),
        (1, Instruction::Newarray(ArrayType::Int)),
        (2, Instruction::Checkcast(CpIndex(1))),
        (5, Instruction::Iconst1),
        (6, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &cp, vec![], 4, 1).unwrap().unwrap();
    assert_eq!(r, Slot::Int(1));
}

// ---- execute(): Instanceof null match arm deletion ----

#[test]
fn execute_instanceof_null_returns_zero() {
    // Mutant: delete null arm → null falls to _ → TypeMismatch error
    use duke_classfile::types::CpIndex;
    let cp = vec![
        None,
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("foo".to_string())),
    ];
    let instructions = vec![
        (0, Instruction::AconstNull),
        (1, Instruction::Instanceof(CpIndex(1))),
        (4, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &cp, vec![], 4, 1).unwrap().unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn execute_instanceof_matching_ref_returns_one() {
    // Kills == → != (matching class → 1; mutant returns 0 when class matches)
    use duke_classfile::types::CpIndex;
    let cp = vec![
        None,
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("[I".to_string())),
    ];
    let instructions = vec![
        (0, Instruction::Bipush(1i8)),
        (1, Instruction::Newarray(ArrayType::Int)),
        (2, Instruction::Instanceof(CpIndex(1))),
        (5, Instruction::Ireturn),
    ];
    let r = execute(&instructions, &cp, vec![], 4, 1).unwrap().unwrap();
    assert_eq!(r, Slot::Int(1));
}

// =========================================================================
// Fifth batch: native function boundary mutant kills
// =========================================================================

// ---- native_string_substring: begin == len should return empty (> → >=) ----

#[test]
fn native_substring_begin_equals_len_returns_empty() {
    // "hello".substring(5) → "" (begin=5=len → valid with >, invalid with >=)
    let mut heap = duke_gc::Heap::new();
    let this = heap.allocate_string("hello".to_string());
    let mut out: Vec<u8> = Vec::new();
    let r = native_string_substring(
        &[Slot::Reference(Some(this)), Slot::Int(5)],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    let sub_ref = r.as_reference().unwrap();
    assert_eq!(heap.get(sub_ref).unwrap().string_value.as_deref(), Some(""));
}

// ---- native_string_substring_range: begin==end returns empty (first > → ==, >= ) ----

#[test]
fn native_substring_range_begin_equals_end_returns_empty() {
    // "hello".substring(2, 2) → "" (begin==end → valid with >, invalid with == or >=)
    let mut heap = duke_gc::Heap::new();
    let this = heap.allocate_string("hello".to_string());
    let mut out: Vec<u8> = Vec::new();
    let r = native_string_substring_range(
        &[Slot::Reference(Some(this)), Slot::Int(2), Slot::Int(2)],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    let sub_ref = r.as_reference().unwrap();
    assert_eq!(heap.get(sub_ref).unwrap().string_value.as_deref(), Some(""));
}

#[test]
fn native_substring_range_end_equals_len_is_valid() {
    // "hello".substring(0, 5) → "hello" (end=5=len → valid with >, invalid with >=)
    let mut heap = duke_gc::Heap::new();
    let this = heap.allocate_string("hello".to_string());
    let mut out: Vec<u8> = Vec::new();
    let r = native_string_substring_range(
        &[Slot::Reference(Some(this)), Slot::Int(0), Slot::Int(5)],
        &mut heap,
        &mut out,
    )
    .unwrap()
    .unwrap();
    let sub_ref = r.as_reference().unwrap();
    assert_eq!(
        heap.get(sub_ref).unwrap().string_value.as_deref(),
        Some("hello")
    );
}

#[test]
fn native_substring_range_begin_greater_than_end_errors() {
    // begin=3 > end=1 → error (first > condition: 3 > 1 → true → error)
    let mut heap = duke_gc::Heap::new();
    let this = heap.allocate_string("hello".to_string());
    let mut out: Vec<u8> = Vec::new();
    let err = native_string_substring_range(
        &[Slot::Reference(Some(this)), Slot::Int(3), Slot::Int(1)],
        &mut heap,
        &mut out,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- format_arg: Long with %X (uppercase) ----

#[test]
fn format_arg_long_uppercase_x_spec() {
    // Mutant: delete Long arm → Long falls to _ → returns "0" instead of "FF"
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Long(255_i64);
    let result = format_arg('X', None, &Slot::Reference(Some(r)), &heap).unwrap();
    assert_eq!(result, "FF");
}

// =========================================================================
// Sixth batch: execute() Ldiv/Lrem + Anewarray + all remaining array types
//              execute_class() Faload/Fastore + byte/char/short/ref arrays
//              Tableswitch, Checkcast null, Multianewarray, parse_arg_count
//              execute_string_concat_recipe + native_string_format
// =========================================================================

// ---- execute(): Ldiv/Lrem success (lines 4129, 4137: == → !=) ----

#[test]
fn execute_ldiv_returns_quotient() {
    // b=1 ≠ 0; correct: 1==0=false → proceed; mutant !=: 1!=0=true → DivisionByZero
    let r = execute(
        &[
            (0, Instruction::Lconst1),
            (1, Instruction::Lconst1),
            (2, Instruction::Ldiv),
            (3, Instruction::Lreturn),
        ],
        &[None],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(1));
}

#[test]
fn execute_lrem_returns_remainder() {
    // b=1 ≠ 0; correct: 1==0=false → proceed; mutant !=: 1!=0=true → DivisionByZero
    let r = execute(
        &[
            (0, Instruction::Lconst1),
            (1, Instruction::Lconst1),
            (2, Instruction::Lrem),
            (3, Instruction::Lreturn),
        ],
        &[None],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Long(0));
}

// ---- execute(): Anewarray count check (line 4498: < → ==, < → >, < → <=) ----

#[test]
fn execute_anewarray_zero_count_succeeds() {
    // count=0: kills < → == (0==0→error) and < → <= (0<=0→error)
    use duke_classfile::types::CpIndex;
    let cp = vec![
        None,
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("java/lang/Object".to_string())),
    ];
    let r = execute(
        &[
            (0, Instruction::Iconst0),
            (1, Instruction::Anewarray(CpIndex(1))),
            (5, Instruction::Pop),
            (6, Instruction::Iconst1),
            (7, Instruction::Ireturn),
        ],
        &cp,
        vec![],
        4,
        1,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_anewarray_one_count_succeeds() {
    // count=1: kills < → > (1>0=true→error; correct: 1<0=false→ok)
    use duke_classfile::types::CpIndex;
    let cp = vec![
        None,
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("java/lang/Object".to_string())),
    ];
    let r = execute(
        &[
            (0, Instruction::Iconst1),
            (1, Instruction::Anewarray(CpIndex(1))),
            (5, Instruction::Pop),
            (6, Instruction::Iconst1),
            (7, Instruction::Ireturn),
        ],
        &cp,
        vec![],
        4,
        1,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_anewarray_negative_count_errors() {
    // count=-1: correct -1<0=true→error; mutant > 0: -1>0=false→no error
    use duke_classfile::types::CpIndex;
    let cp = vec![
        None,
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("java/lang/Object".to_string())),
    ];
    let err = execute(
        &[
            (0, Instruction::IconstM1),
            (1, Instruction::Anewarray(CpIndex(1))),
            (5, Instruction::Pop),
            (6, Instruction::Iconst0),
            (7, Instruction::Ireturn),
        ],
        &cp,
        vec![],
        4,
        1,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::NegativeArraySize { .. }));
}

// ---- execute(): Aaload bounds (line 4663: || → &&, < → ==, < → >, < → <=) ----

#[test]
fn execute_aaload_valid_idx0_returns_null() {
    // idx=0: kills < → == (0==0→error) and < → <= (0<=0→error)
    use duke_classfile::types::CpIndex;
    let cp = vec![
        None,
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("java/lang/Object".to_string())),
    ];
    let r = execute(
        &[
            (0, Instruction::Iconst2),
            (1, Instruction::Anewarray(CpIndex(1))),
            (5, Instruction::Iconst0),
            (6, Instruction::Aaload),
            (7, Instruction::Pop),
            (8, Instruction::Iconst1),
            (9, Instruction::Ireturn),
        ],
        &cp,
        vec![],
        4,
        1,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_aaload_valid_idx1_returns_null() {
    // idx=1: kills < → > (1>0=true→error; correct: 1<0=false→ok)
    use duke_classfile::types::CpIndex;
    let cp = vec![
        None,
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("java/lang/Object".to_string())),
    ];
    let r = execute(
        &[
            (0, Instruction::Iconst2),
            (1, Instruction::Anewarray(CpIndex(1))),
            (5, Instruction::Iconst1),
            (6, Instruction::Aaload),
            (7, Instruction::Pop),
            (8, Instruction::Iconst1),
            (9, Instruction::Ireturn),
        ],
        &cp,
        vec![],
        4,
        1,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_aaload_oob_at_length_errors() {
    // idx=2=length: kills || → && (false && true = false → no error, but panics)
    use duke_classfile::types::CpIndex;
    let cp = vec![
        None,
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("java/lang/Object".to_string())),
    ];
    let err = execute(
        &[
            (0, Instruction::Iconst2),
            (1, Instruction::Anewarray(CpIndex(1))),
            (5, Instruction::Iconst2),
            (6, Instruction::Aaload),
            (7, Instruction::Pop),
            (8, Instruction::Iconst0),
            (9, Instruction::Ireturn),
        ],
        &cp,
        vec![],
        4,
        1,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- execute(): Aastore bounds (line 4680: || → &&, < → ==, < → >, < → <=) ----

#[test]
fn execute_aastore_valid_idx0_stores() {
    // idx=0: kills < → == and < → <=; stack: ref, idx=0, null
    use duke_classfile::types::CpIndex;
    let cp = vec![
        None,
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("java/lang/Object".to_string())),
    ];
    let r = execute(
        &[
            (0, Instruction::Iconst2),
            (1, Instruction::Anewarray(CpIndex(1))),
            (5, Instruction::Dup),
            (6, Instruction::Iconst0),
            (7, Instruction::AconstNull),
            (8, Instruction::Aastore),
            (9, Instruction::Pop),
            (10, Instruction::Iconst1),
            (11, Instruction::Ireturn),
        ],
        &cp,
        vec![],
        6,
        1,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_aastore_valid_idx1_stores() {
    // idx=1: kills < → > (1>0=true→error)
    use duke_classfile::types::CpIndex;
    let cp = vec![
        None,
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("java/lang/Object".to_string())),
    ];
    let r = execute(
        &[
            (0, Instruction::Iconst2),
            (1, Instruction::Anewarray(CpIndex(1))),
            (5, Instruction::Dup),
            (6, Instruction::Iconst1),
            (7, Instruction::AconstNull),
            (8, Instruction::Aastore),
            (9, Instruction::Pop),
            (10, Instruction::Iconst1),
            (11, Instruction::Ireturn),
        ],
        &cp,
        vec![],
        6,
        1,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_aastore_oob_at_length_errors() {
    // idx=2=length: kills || → &&
    use duke_classfile::types::CpIndex;
    let cp = vec![
        None,
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("java/lang/Object".to_string())),
    ];
    let err = execute(
        &[
            (0, Instruction::Iconst2),
            (1, Instruction::Anewarray(CpIndex(1))),
            (5, Instruction::Iconst2),
            (6, Instruction::AconstNull),
            (7, Instruction::Aastore),
            (8, Instruction::Iconst0),
            (9, Instruction::Ireturn),
        ],
        &cp,
        vec![],
        6,
        1,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- execute(): Baload bounds (line 4697: || → &&, < → ==, < → >, < → <=) ----

#[test]
fn execute_baload_valid_idx0_returns_zero() {
    // idx=0: kills < → == (0==0→error) and < → <= (0<=0→error)
    let r = execute(
        &[
            (0, Instruction::Bipush(2i8)),
            (2, Instruction::Newarray(ArrayType::Byte)),
            (4, Instruction::Iconst0),
            (5, Instruction::Baload),
            (6, Instruction::Ireturn),
        ],
        &[None],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn execute_baload_valid_idx1_returns_zero() {
    // idx=1: kills < → > (1>0=true→error; correct: 1<0=false→ok)
    let r = execute(
        &[
            (0, Instruction::Bipush(2i8)),
            (2, Instruction::Newarray(ArrayType::Byte)),
            (4, Instruction::Iconst1),
            (5, Instruction::Baload),
            (6, Instruction::Ireturn),
        ],
        &[None],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn execute_baload_oob_at_length_errors() {
    // idx=2=length: kills || → &&
    let err = execute(
        &[
            (0, Instruction::Bipush(2i8)),
            (2, Instruction::Newarray(ArrayType::Byte)),
            (4, Instruction::Iconst2),
            (5, Instruction::Baload),
            (6, Instruction::Ireturn),
        ],
        &[None],
        vec![],
        4,
        0,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- execute(): Bastore bounds (line 4714: || → &&, < → ==, < → >, < → <=) ----

#[test]
fn execute_bastore_valid_idx0_stores() {
    // idx=0: kills < → == and < → <=; stack: ref, ref, idx=0, val=42
    let r = execute(
        &[
            (0, Instruction::Bipush(2i8)),
            (2, Instruction::Newarray(ArrayType::Byte)),
            (4, Instruction::Dup),
            (5, Instruction::Iconst0),
            (6, Instruction::Bipush(42i8)),
            (8, Instruction::Bastore),
            (9, Instruction::Pop),
            (10, Instruction::Iconst1),
            (11, Instruction::Ireturn),
        ],
        &[None],
        vec![],
        6,
        1,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_bastore_valid_idx1_stores() {
    // idx=1: kills < → > (1>0=true→error)
    let r = execute(
        &[
            (0, Instruction::Bipush(2i8)),
            (2, Instruction::Newarray(ArrayType::Byte)),
            (4, Instruction::Dup),
            (5, Instruction::Iconst1),
            (6, Instruction::Bipush(55i8)),
            (8, Instruction::Bastore),
            (9, Instruction::Pop),
            (10, Instruction::Iconst1),
            (11, Instruction::Ireturn),
        ],
        &[None],
        vec![],
        6,
        1,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_bastore_oob_at_length_errors() {
    // idx=2=length: kills || → &&; stack: ref, idx=2, val=0
    let err = execute(
        &[
            (0, Instruction::Bipush(2i8)),
            (2, Instruction::Newarray(ArrayType::Byte)),
            (4, Instruction::Iconst2),
            (5, Instruction::Bipush(0i8)),
            (7, Instruction::Bastore),
            (8, Instruction::Iconst0),
            (9, Instruction::Ireturn),
        ],
        &[None],
        vec![],
        6,
        1,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- execute(): Caload bounds (line 4731: || → &&, < → ==, < → >, < → <=) ----

#[test]
fn execute_caload_valid_idx0_returns_zero() {
    // idx=0: kills < → == and < → <=
    let r = execute(
        &[
            (0, Instruction::Bipush(2i8)),
            (2, Instruction::Newarray(ArrayType::Char)),
            (4, Instruction::Iconst0),
            (5, Instruction::Caload),
            (6, Instruction::Ireturn),
        ],
        &[None],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn execute_caload_valid_idx1_returns_zero() {
    // idx=1: kills < → > (1>0=true→error)
    let r = execute(
        &[
            (0, Instruction::Bipush(2i8)),
            (2, Instruction::Newarray(ArrayType::Char)),
            (4, Instruction::Iconst1),
            (5, Instruction::Caload),
            (6, Instruction::Ireturn),
        ],
        &[None],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn execute_caload_oob_at_length_errors() {
    // idx=2=length: kills || → &&
    let err = execute(
        &[
            (0, Instruction::Bipush(2i8)),
            (2, Instruction::Newarray(ArrayType::Char)),
            (4, Instruction::Iconst2),
            (5, Instruction::Caload),
            (6, Instruction::Ireturn),
        ],
        &[None],
        vec![],
        4,
        0,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- execute(): Castore bounds (line 4748: || → &&, < → ==, < → >, < → <=) ----

#[test]
fn execute_castore_valid_idx0_stores() {
    // idx=0: kills < → == and < → <=
    let r = execute(
        &[
            (0, Instruction::Bipush(2i8)),
            (2, Instruction::Newarray(ArrayType::Char)),
            (4, Instruction::Dup),
            (5, Instruction::Iconst0),
            (6, Instruction::Bipush(65i8)),
            (8, Instruction::Castore),
            (9, Instruction::Pop),
            (10, Instruction::Iconst1),
            (11, Instruction::Ireturn),
        ],
        &[None],
        vec![],
        6,
        1,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_castore_valid_idx1_stores() {
    // idx=1: kills < → > (1>0=true→error)
    let r = execute(
        &[
            (0, Instruction::Bipush(2i8)),
            (2, Instruction::Newarray(ArrayType::Char)),
            (4, Instruction::Dup),
            (5, Instruction::Iconst1),
            (6, Instruction::Bipush(66i8)),
            (8, Instruction::Castore),
            (9, Instruction::Pop),
            (10, Instruction::Iconst1),
            (11, Instruction::Ireturn),
        ],
        &[None],
        vec![],
        6,
        1,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_castore_oob_at_length_errors() {
    // idx=2=length: kills || → &&; stack: ref, idx=2, val=0
    let err = execute(
        &[
            (0, Instruction::Bipush(2i8)),
            (2, Instruction::Newarray(ArrayType::Char)),
            (4, Instruction::Iconst2),
            (5, Instruction::Iconst0),
            (6, Instruction::Castore),
            (7, Instruction::Iconst0),
            (8, Instruction::Ireturn),
        ],
        &[None],
        vec![],
        6,
        1,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- execute(): Saload bounds (line 4765: || → &&, < → ==, < → >, < → <=) ----

#[test]
fn execute_saload_valid_idx0_returns_zero() {
    // idx=0: kills < → == and < → <=
    let r = execute(
        &[
            (0, Instruction::Bipush(2i8)),
            (2, Instruction::Newarray(ArrayType::Short)),
            (4, Instruction::Iconst0),
            (5, Instruction::Saload),
            (6, Instruction::Ireturn),
        ],
        &[None],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn execute_saload_valid_idx1_returns_zero() {
    // idx=1: kills < → > (1>0=true→error)
    let r = execute(
        &[
            (0, Instruction::Bipush(2i8)),
            (2, Instruction::Newarray(ArrayType::Short)),
            (4, Instruction::Iconst1),
            (5, Instruction::Saload),
            (6, Instruction::Ireturn),
        ],
        &[None],
        vec![],
        4,
        0,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn execute_saload_oob_at_length_errors() {
    // idx=2=length: kills || → &&
    let err = execute(
        &[
            (0, Instruction::Bipush(2i8)),
            (2, Instruction::Newarray(ArrayType::Short)),
            (4, Instruction::Iconst2),
            (5, Instruction::Saload),
            (6, Instruction::Ireturn),
        ],
        &[None],
        vec![],
        4,
        0,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- execute(): Sastore bounds (line 4782: || → &&, < → ==, < → >, < → <=) ----

#[test]
fn execute_sastore_valid_idx0_stores() {
    // idx=0: kills < → == and < → <=
    let r = execute(
        &[
            (0, Instruction::Bipush(2i8)),
            (2, Instruction::Newarray(ArrayType::Short)),
            (4, Instruction::Dup),
            (5, Instruction::Iconst0),
            (6, Instruction::Bipush(10i8)),
            (8, Instruction::Sastore),
            (9, Instruction::Pop),
            (10, Instruction::Iconst1),
            (11, Instruction::Ireturn),
        ],
        &[None],
        vec![],
        6,
        1,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_sastore_valid_idx1_stores() {
    // idx=1: kills < → > (1>0=true→error)
    let r = execute(
        &[
            (0, Instruction::Bipush(2i8)),
            (2, Instruction::Newarray(ArrayType::Short)),
            (4, Instruction::Dup),
            (5, Instruction::Iconst1),
            (6, Instruction::Bipush(20i8)),
            (8, Instruction::Sastore),
            (9, Instruction::Pop),
            (10, Instruction::Iconst1),
            (11, Instruction::Ireturn),
        ],
        &[None],
        vec![],
        6,
        1,
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn execute_sastore_oob_at_length_errors() {
    // idx=2=length: kills || → &&; stack: ref, idx=2, val=0
    let err = execute(
        &[
            (0, Instruction::Bipush(2i8)),
            (2, Instruction::Newarray(ArrayType::Short)),
            (4, Instruction::Iconst2),
            (5, Instruction::Iconst0),
            (6, Instruction::Sastore),
            (7, Instruction::Iconst0),
            (8, Instruction::Ireturn),
        ],
        &[None],
        vec![],
        6,
        1,
    )
    .unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// =====================================================================
// Sixth batch: execute_class() array bounds
// =====================================================================

// ---- execute_class(): Faload valid idx=1 (line 6881: < → >) ----

#[test]
fn ec_faload_valid_idx1_returns_value() {
    // idx=1: kills < → > (1>0=true→error; correct: 1<0=false→ok)
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Float)),
        (2, Instruction::Iconst1),
        (3, Instruction::Faload),
        (4, Instruction::Pop),
        (5, Instruction::Iconst1),
        (6, Instruction::Ireturn),
    ];
    let r = execute_class_synthetic(instructions, vec![], 4, 0, "()I")
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(1));
}

// ---- execute_class(): Fastore valid idx=0, idx=1 (line 6897: < → ==, < → >, < → <=) ----

#[test]
fn ec_fastore_valid_idx0_stores() {
    // idx=0: kills < → == (0==0→error) and < → <= (0<=0→error)
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Float)),
        (2, Instruction::Dup),
        (3, Instruction::Iconst0),
        (4, Instruction::Fconst0),
        (5, Instruction::Fastore),
        (6, Instruction::Pop),
        (7, Instruction::Iconst1),
        (8, Instruction::Ireturn),
    ];
    let r = execute_class_synthetic(instructions, vec![], 6, 0, "()I")
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn ec_fastore_valid_idx1_stores() {
    // idx=1: kills < → > (1>0=true→error)
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Float)),
        (2, Instruction::Dup),
        (3, Instruction::Iconst1),
        (4, Instruction::Fconst0),
        (5, Instruction::Fastore),
        (6, Instruction::Pop),
        (7, Instruction::Iconst1),
        (8, Instruction::Ireturn),
    ];
    let r = execute_class_synthetic(instructions, vec![], 6, 0, "()I")
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(1));
}

// ---- execute_class(): Aaload/Aastore OOB (lines 6945, 6961: || → &&) ----

#[test]
fn ec_aaload_oob_at_length_errors() {
    // Use int array as stand-in; Aaload clones element without type-checking
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Int)),
        (2, Instruction::Iconst2),
        (3, Instruction::Aaload),
        (4, Instruction::Pop),
        (5, Instruction::Iconst0),
        (6, Instruction::Ireturn),
    ];
    let err = execute_class_synthetic(instructions, vec![], 4, 0, "()I").unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

#[test]
fn ec_aastore_oob_at_length_errors() {
    // idx=2=length: kills || → &&; stack: ref, idx=2, null
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Int)),
        (2, Instruction::Iconst2),
        (3, Instruction::AconstNull),
        (4, Instruction::Aastore),
        (5, Instruction::Iconst0),
        (6, Instruction::Ireturn),
    ];
    let err = execute_class_synthetic(instructions, vec![], 4, 0, "()I").unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- execute_class(): Baload bounds (line 6977: || → &&, < → >, < → ==, < → <=) ----

#[test]
fn ec_baload_valid_idx0_returns_zero() {
    // idx=0: kills < → == and < → <=
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Byte)),
        (2, Instruction::Iconst0),
        (3, Instruction::Baload),
        (4, Instruction::Ireturn),
    ];
    let r = execute_class_synthetic(instructions, vec![], 4, 0, "()I")
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn ec_baload_valid_idx1_returns_zero() {
    // idx=1: kills < → > (1>0=true→error)
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Byte)),
        (2, Instruction::Iconst1),
        (3, Instruction::Baload),
        (4, Instruction::Ireturn),
    ];
    let r = execute_class_synthetic(instructions, vec![], 4, 0, "()I")
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn ec_baload_oob_at_length_errors() {
    // idx=2=length: kills || → &&
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Byte)),
        (2, Instruction::Iconst2),
        (3, Instruction::Baload),
        (4, Instruction::Ireturn),
    ];
    let err = execute_class_synthetic(instructions, vec![], 4, 0, "()I").unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- execute_class(): Bastore bounds (line 6993: < → >, || → &&, < → ==, < → <=) ----

#[test]
fn ec_bastore_valid_idx0_stores() {
    // idx=0: kills < → == and < → <=
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Byte)),
        (2, Instruction::Dup),
        (3, Instruction::Iconst0),
        (4, Instruction::Bipush(42i8)),
        (6, Instruction::Bastore),
        (7, Instruction::Pop),
        (8, Instruction::Iconst1),
        (9, Instruction::Ireturn),
    ];
    let r = execute_class_synthetic(instructions, vec![], 6, 0, "()I")
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn ec_bastore_valid_idx1_stores() {
    // idx=1: kills < → > (1>0=true→error)
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Byte)),
        (2, Instruction::Dup),
        (3, Instruction::Iconst1),
        (4, Instruction::Bipush(55i8)),
        (6, Instruction::Bastore),
        (7, Instruction::Pop),
        (8, Instruction::Iconst1),
        (9, Instruction::Ireturn),
    ];
    let r = execute_class_synthetic(instructions, vec![], 6, 0, "()I")
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn ec_bastore_oob_at_length_errors() {
    // idx=2=length: kills || → &&; stack: ref, idx=2, val=0
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Byte)),
        (2, Instruction::Iconst2),
        (3, Instruction::Bipush(0i8)),
        (5, Instruction::Bastore),
        (6, Instruction::Iconst0),
        (7, Instruction::Ireturn),
    ];
    let err = execute_class_synthetic(instructions, vec![], 6, 0, "()I").unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- execute_class(): Caload OOB (line 7009: || → &&) ----

#[test]
fn ec_caload_oob_at_length_errors() {
    // idx=2=length: kills || → &&
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Char)),
        (2, Instruction::Iconst2),
        (3, Instruction::Caload),
        (4, Instruction::Ireturn),
    ];
    let err = execute_class_synthetic(instructions, vec![], 4, 0, "()I").unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- execute_class(): Castore bounds (line 7025: || → &&, < → ==, < → >, < → <=) ----

#[test]
fn ec_castore_valid_idx0_stores() {
    // idx=0: kills < → == and < → <=
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Char)),
        (2, Instruction::Dup),
        (3, Instruction::Iconst0),
        (4, Instruction::Bipush(65i8)),
        (6, Instruction::Castore),
        (7, Instruction::Pop),
        (8, Instruction::Iconst1),
        (9, Instruction::Ireturn),
    ];
    let r = execute_class_synthetic(instructions, vec![], 6, 0, "()I")
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn ec_castore_valid_idx1_stores() {
    // idx=1: kills < → > (1>0=true→error)
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Char)),
        (2, Instruction::Dup),
        (3, Instruction::Iconst1),
        (4, Instruction::Bipush(66i8)),
        (6, Instruction::Castore),
        (7, Instruction::Pop),
        (8, Instruction::Iconst1),
        (9, Instruction::Ireturn),
    ];
    let r = execute_class_synthetic(instructions, vec![], 6, 0, "()I")
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn ec_castore_oob_at_length_errors() {
    // idx=2=length: kills || → &&; stack: ref, idx=2, val=0
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Char)),
        (2, Instruction::Iconst2),
        (3, Instruction::Iconst0),
        (4, Instruction::Castore),
        (5, Instruction::Iconst0),
        (6, Instruction::Ireturn),
    ];
    let err = execute_class_synthetic(instructions, vec![], 6, 0, "()I").unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- execute_class(): Saload bounds (line 7041: || → &&, < → ==, < → <=, < → >) ----

#[test]
fn ec_saload_valid_idx0_returns_zero() {
    // idx=0: kills < → == and < → <=
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Short)),
        (2, Instruction::Iconst0),
        (3, Instruction::Saload),
        (4, Instruction::Ireturn),
    ];
    let r = execute_class_synthetic(instructions, vec![], 4, 0, "()I")
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn ec_saload_valid_idx1_returns_zero() {
    // idx=1: kills < → > (1>0=true→error)
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Short)),
        (2, Instruction::Iconst1),
        (3, Instruction::Saload),
        (4, Instruction::Ireturn),
    ];
    let r = execute_class_synthetic(instructions, vec![], 4, 0, "()I")
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(0));
}

#[test]
fn ec_saload_oob_at_length_errors() {
    // idx=2=length: kills || → &&
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Short)),
        (2, Instruction::Iconst2),
        (3, Instruction::Saload),
        (4, Instruction::Ireturn),
    ];
    let err = execute_class_synthetic(instructions, vec![], 4, 0, "()I").unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- execute_class(): Sastore bounds (line 7057: || → &&, < → ==, < → >, < → <=) ----

#[test]
fn ec_sastore_valid_idx0_stores() {
    // idx=0: kills < → == and < → <=
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Short)),
        (2, Instruction::Dup),
        (3, Instruction::Iconst0),
        (4, Instruction::Bipush(10i8)),
        (6, Instruction::Sastore),
        (7, Instruction::Pop),
        (8, Instruction::Iconst1),
        (9, Instruction::Ireturn),
    ];
    let r = execute_class_synthetic(instructions, vec![], 6, 0, "()I")
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn ec_sastore_valid_idx1_stores() {
    // idx=1: kills < → > (1>0=true→error)
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Short)),
        (2, Instruction::Dup),
        (3, Instruction::Iconst1),
        (4, Instruction::Bipush(20i8)),
        (6, Instruction::Sastore),
        (7, Instruction::Pop),
        (8, Instruction::Iconst1),
        (9, Instruction::Ireturn),
    ];
    let r = execute_class_synthetic(instructions, vec![], 6, 0, "()I")
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(1));
}

#[test]
fn ec_sastore_oob_at_length_errors() {
    // idx=2=length: kills || → &&; stack: ref, idx=2, val=0
    let instructions = vec![
        (0, Instruction::Iconst2),
        (1, Instruction::Newarray(ArrayType::Short)),
        (2, Instruction::Iconst2),
        (3, Instruction::Iconst0),
        (4, Instruction::Sastore),
        (5, Instruction::Iconst0),
        (6, Instruction::Ireturn),
    ];
    let err = execute_class_synthetic(instructions, vec![], 6, 0, "()I").unwrap_err();
    assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
}

// ---- execute_class(): Tableswitch nonzero low (line 7076: - → +) ----

#[test]
fn ec_tableswitch_nonzero_low_matches_key() {
    // key=2, low=1, high=3: offsets[key-low=1]=9 → target_pc=1+9=10 → Bipush(99)
    // Mutant - → +: offsets[(2+1)=3] → index 3 OOB panic
    let instructions = vec![
        (0, Instruction::Iconst2),
        (
            1,
            Instruction::Tableswitch {
                default: 11i32, // 1+11=12 → default path
                low: 1i32,
                high: 3i32,
                offsets: vec![11i32, 9i32, 11i32], // key=1→12, key=2→10, key=3→12
            },
        ),
        (10, Instruction::Bipush(99i8)),
        (11, Instruction::Ireturn),
        (12, Instruction::Iconst0),
        (13, Instruction::Ireturn),
    ];
    let r = execute_class_synthetic(instructions, vec![], 4, 0, "()I")
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(99));
}

// ---- execute_class(): Checkcast null arm deletion (line 7095) ----

#[test]
fn ec_checkcast_null_passes() {
    // Kills: delete Slot::Reference(None) arm → null falls to _ → TypeMismatch
    // Null arm exits before CP lookup, so CpIndex(0) (None entry) is safe here
    use duke_classfile::types::CpIndex;
    let instructions = vec![
        (0, Instruction::AconstNull),
        (1, Instruction::Checkcast(CpIndex(0))),
        (5, Instruction::Iconst1),
        (6, Instruction::Ireturn),
    ];
    let r = execute_class_synthetic(instructions, vec![], 4, 1, "()I")
        .unwrap()
        .unwrap();
    assert_eq!(r, Slot::Int(1));
}

// ---- execute_class(): Multianewarray negative dim (line 7863: < → ==, < → <=) ----

#[test]
fn ec_multianewarray_negative_dim_errors() {
    // dim=-1: kills < → == (-1==0=false→no error; correct: -1<0=true→error)
    use duke_classfile::types::CpIndex;
    use std::sync::Arc;
    let instructions = vec![
        (0, Instruction::IconstM1),
        (
            1,
            Instruction::Multianewarray {
                index: CpIndex(1),
                dimensions: 1,
            },
        ),
        (4, Instruction::Pop),
        (5, Instruction::Iconst0),
        (6, Instruction::Ireturn),
    ];
    let pc_to_idx: std::collections::HashMap<usize, usize> = instructions
        .iter()
        .enumerate()
        .map(|(i, (pc, _))| (*pc, i))
        .collect();
    let method = MethodEntry {
        name: "syntest".to_string(),
        descriptor: "()I".to_string(),
        is_public: true,
        is_static: true,
        instructions: instructions.into(),
        max_stack: 4,
        max_locals: 1,
        exception_table: vec![],
        pc_to_idx: Arc::new(pc_to_idx),
    };
    let cp = vec![
        None,
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("[[I".to_string())),
    ];
    let ctx = ClassContext {
        class_name: "SynTest".to_string(),
        super_class: None,
        interfaces: vec![],
        constant_pool: cp,
        methods: vec![method],
        fields: vec![],
        static_fields: vec![],
        instance_field_count: 0,
        bootstrap_methods: vec![],
    };
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = make_simple_loader();
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let err = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "SynTest",
        "syntest",
        "()I",
        &[],
    )
    .unwrap_err();
    assert!(matches!(err, VmError::NegativeArraySize { .. }));
}

#[test]
fn ec_multianewarray_zero_dim_succeeds() {
    // dim=0: kills < → <= (0<=0=true→error; correct: 0<0=false→ok)
    use duke_classfile::types::CpIndex;
    use std::sync::Arc;
    let instructions = vec![
        (0, Instruction::Iconst0),
        (
            1,
            Instruction::Multianewarray {
                index: CpIndex(1),
                dimensions: 1,
            },
        ),
        (4, Instruction::Pop),
        (5, Instruction::Iconst1),
        (6, Instruction::Ireturn),
    ];
    let pc_to_idx: std::collections::HashMap<usize, usize> = instructions
        .iter()
        .enumerate()
        .map(|(i, (pc, _))| (*pc, i))
        .collect();
    let method = MethodEntry {
        name: "syntest".to_string(),
        descriptor: "()I".to_string(),
        is_public: true,
        is_static: true,
        instructions: instructions.into(),
        max_stack: 4,
        max_locals: 1,
        exception_table: vec![],
        pc_to_idx: Arc::new(pc_to_idx),
    };
    let cp = vec![
        None,
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("[[I".to_string())),
    ];
    let ctx = ClassContext {
        class_name: "SynTest".to_string(),
        super_class: None,
        interfaces: vec![],
        constant_pool: cp,
        methods: vec![method],
        fields: vec![],
        static_fields: vec![],
        instance_field_count: 0,
        bootstrap_methods: vec![],
    };
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = make_simple_loader();
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let r = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "SynTest",
        "syntest",
        "()I",
        &[],
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

// ---- parse_arg_count/parse_arg_types fuzzing ----

use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_parse_arg_count(ref s in "\\PC*") {
        let _ = parse_arg_count(s);
    }

    #[test]
    fn fuzz_parse_arg_types(ref s in "\\PC*") {
        let _ = parse_arg_types(s);
    }
}

// ---- parse_arg_count: array arm deletion (line 8295) ----

#[test]
fn parse_arg_count_primitive_array_param() {
    // ([I)V has 1 param; mutant deletes '[' arm → counts 0
    assert_eq!(parse_arg_count("([I)V"), 1);
}

#[test]
fn parse_arg_count_object_array_param() {
    // ([Ljava/lang/String;)V has 1 param; mutant: falls to _, count stays 0
    assert_eq!(parse_arg_count("([Ljava/lang/String;)V"), 1);
}

#[test]
fn parse_arg_count_two_array_params() {
    // ([I[J)V has 2 params; mutant: 0
    assert_eq!(parse_arg_count("([I[J)V"), 2);
}

// ---- execute_string_concat_recipe (lines 3641, 3648, 3650) ----

#[test]
fn string_concat_recipe_extra_dynamic_placeholder_silently_skipped() {
    // Recipe "\u{1}\u{1}" with 1 arg: second \u{1} → dyn_idx=1 < len=1 is false → skip
    // Mutant < → <=: 1 <= 1 → true → dynamic_args[1] OOB panic
    let mut heap = duke_gc::Heap::new();
    let r =
        execute_string_concat_recipe("\u{1}\u{1}", &[Slot::Int(42)], &['I'], &[], &mut heap)
            .unwrap();
    let s = heap
        .get(r.as_reference().unwrap())
        .unwrap()
        .string_value
        .as_deref()
        .unwrap();
    assert_eq!(s, "42");
}

#[test]
fn string_concat_recipe_extra_constant_placeholder_silently_skipped() {
    // Recipe "\u{2}\u{2}" with 1 constant: second \u{2} → const_idx=1 < len=1 is false → skip
    // Mutant < → <=: 1 <= 1 → true → constants[1] OOB panic
    let mut heap = duke_gc::Heap::new();
    let r =
        execute_string_concat_recipe("\u{2}\u{2}", &[], &[], &["hello".to_string()], &mut heap)
            .unwrap();
    let s = heap
        .get(r.as_reference().unwrap())
        .unwrap()
        .string_value
        .as_deref()
        .unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn string_concat_recipe_two_constants_both_appended() {
    // Recipe "\u{2}\u{2}" with constants ["A","B"] → "AB"
    // Mutant += → *=: const_idx stays 0 → "AA"
    let mut heap = duke_gc::Heap::new();
    let r = execute_string_concat_recipe(
        "\u{2}\u{2}",
        &[],
        &[],
        &["A".to_string(), "B".to_string()],
        &mut heap,
    )
    .unwrap();
    let s = heap
        .get(r.as_reference().unwrap())
        .unwrap()
        .string_value
        .as_deref()
        .unwrap();
    assert_eq!(s, "AB");
}

// ---- native_string_format (lines 2877, 2891, 2904) ----

#[test]
fn native_string_format_width_digit_consumed() {
    // "%1s" with arg "hi": width digit '1' consumed → spec='s' → "hi"
    // Mutant < → == at width loop: loop never runs → spec='1' → wrong output
    let mut heap = duke_gc::Heap::new();
    let fmt = heap.allocate_string("%1s".to_string());
    let arr = heap.allocate("[Ljava/lang/Object;".to_string(), 1);
    let elem = heap.allocate_string("hi".to_string());
    heap.get_mut(arr).unwrap().fields[0] = Slot::Reference(Some(elem));
    let mut sink: Vec<u8> = Vec::new();
    let r = native_string_format(
        &[Slot::Reference(Some(fmt)), Slot::Reference(Some(arr))],
        &mut heap,
        &mut sink,
    )
    .unwrap()
    .unwrap();
    let result = heap
        .get(r.as_reference().unwrap())
        .unwrap()
        .string_value
        .clone()
        .unwrap();
    assert_eq!(result, "hi");
}

#[test]
fn native_string_format_extra_spec_gets_null_arg() {
    // "%s%s" with 1 arg "A": second %s → null (arg_idx=1 not < arr_len=1)
    // Mutant < → <=: arg_idx=1 <= 1 → true → fields[1] OOB panic
    let mut heap = duke_gc::Heap::new();
    let fmt = heap.allocate_string("%s%s".to_string());
    let arr = heap.allocate("[Ljava/lang/Object;".to_string(), 1);
    let a = heap.allocate_string("A".to_string());
    heap.get_mut(arr).unwrap().fields[0] = Slot::Reference(Some(a));
    let mut sink: Vec<u8> = Vec::new();
    let r = native_string_format(
        &[Slot::Reference(Some(fmt)), Slot::Reference(Some(arr))],
        &mut heap,
        &mut sink,
    )
    .unwrap()
    .unwrap();
    let result = heap
        .get(r.as_reference().unwrap())
        .unwrap()
        .string_value
        .clone()
        .unwrap();
    assert_eq!(result, "Anull");
}

#[test]
fn native_string_format_arg_idx_advances_between_specs() {
    // "%s%s" with args ["X","Y"] → "XY"; mutant += → *=: arg_idx stays 0 → "XX"
    let mut heap = duke_gc::Heap::new();
    let fmt = heap.allocate_string("%s%s".to_string());
    let arr = heap.allocate("[Ljava/lang/Object;".to_string(), 2);
    let x = heap.allocate_string("X".to_string());
    let y = heap.allocate_string("Y".to_string());
    heap.get_mut(arr).unwrap().fields[0] = Slot::Reference(Some(x));
    heap.get_mut(arr).unwrap().fields[1] = Slot::Reference(Some(y));
    let mut sink: Vec<u8> = Vec::new();
    let r = native_string_format(
        &[Slot::Reference(Some(fmt)), Slot::Reference(Some(arr))],
        &mut heap,
        &mut sink,
    )
    .unwrap()
    .unwrap();
    let result = heap
        .get(r.as_reference().unwrap())
        .unwrap()
        .string_value
        .clone()
        .unwrap();
    assert_eq!(result, "XY");
}

// ---- execute_class: Idiv b==0 check (line 5856 == → !=) ----

#[test]
fn ec_idiv_nonzero_denominator_succeeds() {
    // kills == → !=: mutation makes nonzero denominator trigger DivisionByZero
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(10i8)),
            (2, Instruction::Bipush(3i8)),
            (4, Instruction::Idiv),
            (5, Instruction::Ireturn),
        ],
        vec![],
        4,
        0,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(3));
}

// ---- execute_class: Ishl bit mask (lines 5876 & → | and & → ^) ----

#[test]
fn ec_ishl_masks_shift_count() {
    // a=1, s=1: 1 << (1 & 31 = 1) = 2
    // & → |: 1 << (1 | 31 = 31) = i32::MIN ≠ 2
    // & → ^: 1 << (1 ^ 31 = 30) = 2^30 ≠ 2
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iconst1),
            (1, Instruction::Iconst1),
            (2, Instruction::Ishl),
            (3, Instruction::Ireturn),
        ],
        vec![],
        4,
        0,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(2));
}

// ---- execute_class: Ishr bit mask (lines 5881 & → | and & → ^) ----

#[test]
fn ec_ishr_masks_shift_count() {
    // a=-4, s=1: (-4) >> (1 & 31 = 1) = -2
    // & → |: (-4) >> (1 | 31 = 31) = -1 ≠ -2
    // & → ^: (-4) >> (1 ^ 31 = 30) = -1 ≠ -2
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(-4i8)),
            (2, Instruction::Iconst1),
            (3, Instruction::Ishr),
            (4, Instruction::Ireturn),
        ],
        vec![],
        4,
        0,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(-2));
}

// ---- execute_class: Fcmpg a < b (line 6026 < → >) ----

#[test]
fn ec_fcmpg_a_less_than_b_returns_minus_one() {
    // 0.0f < 1.0f via Fcmpg: correct = -1
    // < → >: else if a > b fires for NaN arm → returns +1 for Fcmpg
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Fconst0),
            (1, Instruction::Fconst1),
            (2, Instruction::Fcmpg),
            (3, Instruction::Ireturn),
        ],
        vec![],
        4,
        0,
        "()I",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(-1));
}

// ---- execute_class: LdcW String (line 5624 delete Utf8 arm) ----

#[test]
fn ec_ldcw_string_pushes_nonnull_ref() {
    use duke_classfile::types::CpIndex;
    use std::sync::Arc;
    // CP: [0]=None, [1]=String{string_index:2}, [2]=Utf8("hi")
    let cp = vec![
        None,
        Some(CpEntry::String {
            string_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("hi".to_string())),
    ];
    // LdcW → Pop (discards) → Iconst1 → Ireturn
    // With Utf8 arm deletion: LdcW errors → test fails → caught
    let pc_to_idx: std::collections::HashMap<usize, usize> =
        [(0, 0), (3, 1), (4, 2), (5, 3)].iter().copied().collect();
    let method = MethodEntry {
        name: "syntest".to_string(),
        descriptor: "()I".to_string(),
        is_public: true,
        is_static: true,
        instructions: std::sync::Arc::from(
            vec![
                (0, Instruction::LdcW(CpIndex(1))),
                (3, Instruction::Pop),
                (4, Instruction::Iconst1),
                (5, Instruction::Ireturn),
            ]
            .into_boxed_slice(),
        ),
        max_stack: 4,
        max_locals: 0,
        exception_table: vec![],
        pc_to_idx: Arc::new(pc_to_idx),
    };
    let ctx = ClassContext {
        class_name: "SynTest".to_string(),
        super_class: None,
        interfaces: vec![],
        constant_pool: cp,
        methods: vec![method],
        fields: vec![],
        static_fields: vec![],
        instance_field_count: 0,
        bootstrap_methods: vec![],
    };
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = make_simple_loader();
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let r = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "SynTest",
        "syntest",
        "()I",
        &[],
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

// ---- execute_class: LdcW Class (line 5653 delete Utf8 arm) ----

#[test]
fn ec_ldcw_class_constant_pushes_nonnull_ref() {
    use duke_classfile::types::CpIndex;
    use std::sync::Arc;
    // CP: [0]=None, [1]=Class{name_index:2}, [2]=Utf8("java/lang/Object")
    let cp = vec![
        None,
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("java/lang/Object".to_string())),
    ];
    // LdcW(Class) → Pop → Iconst1 → Ireturn
    // With Utf8 arm deletion in LdcW Class branch: class_info=None → ldc_push with Class entry → InvalidCpIndex
    let pc_to_idx: std::collections::HashMap<usize, usize> =
        [(0, 0), (3, 1), (4, 2), (5, 3)].iter().copied().collect();
    let method = MethodEntry {
        name: "syntest".to_string(),
        descriptor: "()I".to_string(),
        is_public: true,
        is_static: true,
        instructions: std::sync::Arc::from(
            vec![
                (0, Instruction::LdcW(CpIndex(1))),
                (3, Instruction::Pop),
                (4, Instruction::Iconst1),
                (5, Instruction::Ireturn),
            ]
            .into_boxed_slice(),
        ),
        max_stack: 4,
        max_locals: 0,
        exception_table: vec![],
        pc_to_idx: Arc::new(pc_to_idx),
    };
    let ctx = ClassContext {
        class_name: "SynTest".to_string(),
        super_class: None,
        interfaces: vec![],
        constant_pool: cp,
        methods: vec![method],
        fields: vec![],
        static_fields: vec![],
        instance_field_count: 0,
        bootstrap_methods: vec![],
    };
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let loader = make_simple_loader();
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let r = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "SynTest",
        "syntest",
        "()I",
        &[],
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, Slot::Int(1));
}

// ---- init_object_fields (lines 8497,8513,8516,8518,8522) ----

#[allow(clippy::too_many_lines)]
#[test]
fn ec_new_initialises_reference_field_to_null() {
    // Class "RefBox" with: 1 static int field + 2 instance fields (int then reference).
    // After `new RefBox`, getfield refField should return Reference(None), not Int(0).
    //
    // Kills:
    //   8497 (skip init): refField stays Int(0) → ifnull fails → return 0
    //   8513 (include statics in slot count): static offset shifts, refField goes OOB → not written
    //   8516 (write only Int(0) defaults): refField (Reference type) skipped → stays Int(0)
    //   8518 (< → ==): slot_idx never == len → never writes → stays Int(0)
    //   8518 (< → >): 0 > 2 = false → never writes
    //   8522 (+= → *=1): slot_idx stays 0, writes to intField slot → refField unchanged
    use duke_classfile::types::CpIndex;
    use std::sync::Arc;

    // CP for "SynTest" calling class:
    // [0]=None, [1]=Class("RefBox"), [2]=Utf8("RefBox"),
    // [3]=Fieldref(class=1, nat=4), [4]=NameAndType(name=5,desc=6),
    // [5]=Utf8("refField"), [6]=Utf8("Ljava/lang/Object;")
    let cp = vec![
        None,
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("RefBox".to_string())),
        Some(CpEntry::Fieldref {
            class_index: CpIndex(1),
            name_and_type_index: CpIndex(4),
        }),
        Some(CpEntry::NameAndType {
            name_index: CpIndex(5),
            descriptor_index: CpIndex(6),
        }),
        Some(CpEntry::Utf8("refField".to_string())),
        Some(CpEntry::Utf8("Ljava/lang/Object;".to_string())),
    ];

    // Instructions:
    // 0: new RefBox
    // 3: getfield refField → pushes fields[1] (slot 1 of instance)
    // 6: ifnull 4           → if null: jump to 6+4=10 (success: field is null as expected)
    // 8: iconst0            → not null: fail
    // 9: ireturn
    // 10: iconst1           → success
    // 11: ireturn
    let instructions = vec![
        (0usize, Instruction::New(CpIndex(1))),
        (3, Instruction::Getfield(CpIndex(3))),
        (6, Instruction::Ifnull(4i16)),
        (8, Instruction::Iconst0),
        (9, Instruction::Ireturn),
        (10, Instruction::Iconst1),
        (11, Instruction::Ireturn),
    ];
    let pc_to_idx: std::collections::HashMap<usize, usize> = instructions
        .iter()
        .enumerate()
        .map(|(i, (pc, _))| (*pc, i))
        .collect();
    let method = MethodEntry {
        name: "syntest".to_string(),
        descriptor: "()I".to_string(),
        is_public: true,
        is_static: true,
        instructions: instructions.into(),
        max_stack: 4,
        max_locals: 0,
        exception_table: vec![],
        pc_to_idx: Arc::new(pc_to_idx),
    };
    let caller_ctx = ClassContext {
        class_name: "SynTest".to_string(),
        super_class: None,
        interfaces: vec![],
        constant_pool: cp,
        methods: vec![method],
        fields: vec![],
        static_fields: vec![],
        instance_field_count: 0,
        bootstrap_methods: vec![],
    };

    // RefBox: 1 static field "count:I" + 2 instance fields "intField:I", "refField:Ljava/lang/Object;"
    let refbox_ctx = ClassContext {
        class_name: "RefBox".to_string(),
        super_class: None,
        interfaces: vec![],
        constant_pool: vec![None],
        methods: vec![],
        fields: vec![
            FieldEntry {
                name: "count".to_string(),
                descriptor: "I".to_string(),
                is_static: true,
            },
            FieldEntry {
                name: "intField".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "refField".to_string(),
                descriptor: "Ljava/lang/Object;".to_string(),
                is_static: false,
            },
        ],
        static_fields: vec![Slot::Int(0)],
        instance_field_count: 2,
        bootstrap_methods: vec![],
    };

    let mut registry = ClassRegistry::new();
    registry.register(caller_ctx);
    registry.register(refbox_ctx);
    let loader = make_simple_loader();
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let r = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "SynTest",
        "syntest",
        "()I",
        &[],
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        r,
        Slot::Int(1),
        "refField should be Reference(None) after init"
    );
}

// ---- array_list_sort negative size (lines 9074 guard, 9075 arm deletion) ----

#[test]
fn array_list_sort_negative_size_returns_error() {
    // Allocate an ArrayList-like object with fields[0] = -1 (negative size).
    // kills 9074 (guard *n >= 0 → true): negative n accepted as huge usize →
    //   elems.len() != size → InvalidRef (not NegativeArraySize)
    // kills 9075 (delete second arm): negative n falls to _ → Ok(None), not error
    let mut heap = duke_gc::Heap::new();
    let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    heap.get_mut(list_ref).unwrap().fields[0] = Slot::Int(-1);
    let mut sink: Vec<u8> = Vec::new();
    let args = [Slot::Reference(Some(list_ref)), Slot::Reference(None)];
    let mut ops = NoopCallbackOps;
    let err = array_list_sort(&args, &mut heap, &mut sink, &mut ops).unwrap_err();
    assert!(
        matches!(err, VmError::NegativeArraySize { .. }),
        "expected NegativeArraySize, got {err:?}"
    );
}

// ---- Phase 30: Reflection fixture coverage ----

#[test]
fn reflection_for_name_and_get_name() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "forNameAndGetName", "()I"),
        1
    );
}

#[test]
fn reflection_string_class_literal_uses_binary_name() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "stringClassLiteralUsesBinaryName",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_declared_methods_include_public_and_private() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "declaredMethodsIncludePublicAndPrivate",
            "()I",
        ),
        5
    );
}

#[test]
fn reflection_declared_fields_include_public_and_private() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "declaredFieldsIncludePublicAndPrivate",
            "()I",
        ),
        2
    );
}

#[test]
fn reflection_invoke_static_and_instance_methods() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "invokeStaticAdd", "()I"),
        7
    );
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "invokeInstanceTimes", "()I"),
        21
    );
}

#[test]
fn reflection_invoke_long_primitive_round_trips() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "invokeStaticAddLong", "()I"),
        1
    );
}

#[test]
fn reflection_missing_class_raises_class_not_found() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "missingClassRaisesClassNotFound",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_private_method_invoke_raises_illegal_access() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "privateMethodRaisesIllegalAccess",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_private_method_invoke_with_accessible_succeeds() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "privateMethodInvokeWithAccessibleSucceeds",
            "()I"
        ),
        13
    );
}

#[test]
fn reflection_target_exception_is_wrapped() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "targetExceptionIsWrapped", "()I"),
        1
    );
}

#[test]
fn reflection_public_field_get_returns_boxed_value() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "getPublicFieldValue", "()I"),
        7
    );
}

#[test]
fn reflection_public_field_set_updates_target() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "setPublicFieldValue", "()I"),
        1
    );
}

#[test]
fn reflection_private_field_get_raises_illegal_access() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "privateFieldGetRaisesIllegalAccess",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_private_field_get_with_accessible_succeeds() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "privateFieldGetWithAccessibleSucceeds",
            "()I"
        ),
        11
    );
}

#[test]
fn reflection_private_field_set_with_accessible_writes_back() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "privateFieldSetWithAccessibleWritesBack",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_field_get_rejects_wrong_target_type() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "publicFieldWrongTargetRaisesIllegalArgument",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_declared_field_by_name_finds_public_and_private() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "declaredFieldByNameFindsPublicAndPrivate",
            "()I",
        ),
        2
    );
}

#[test]
fn reflection_missing_declared_field_raises_no_such_field() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "missingDeclaredFieldRaisesNoSuchField",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_get_field_finds_inherited_public_superclass_member() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "inheritedPublicFieldLookupFindsSuperclassMember",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_get_method_finds_inherited_public_superclass_member() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "inheritedPublicMethodLookupFindsSuperclassMember",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_get_field_skips_inherited_private_superclass_member() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "inheritedPrivateFieldIsNotPublicLookupVisible",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_get_method_skips_inherited_private_superclass_member() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "inheritedPrivateMethodIsNotPublicLookupVisible",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_get_fields_includes_declared_and_inherited_public_only() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "publicFieldsIncludeDeclaredAndInheritedPublicOnly",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_get_methods_includes_declared_and_inherited_public_only() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "publicMethodsIncludeDeclaredAndInheritedPublicOnly",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_field_get_declaring_class_matches_declared_and_inherited_owners() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "fieldDeclaringClassMatchesDeclaredAndInheritedOwners",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_method_get_declaring_class_matches_declared_and_inherited_owners() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "methodDeclaringClassMatchesDeclaredAndInheritedOwners",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_public_constructor_new_instance_creates_object() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "publicConstructorNewInstanceCreatesObject",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_public_constructor_lookup_skips_private_constructor() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "publicConstructorLookupSkipsPrivateConstructor",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_private_declared_constructor_with_accessible_creates_object() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "privateDeclaredConstructorWithAccessibleCreatesObject",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_declared_constructors_include_public_and_private() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "declaredConstructorsIncludePublicAndPrivate",
            "()I",
        ),
        2
    );
}

#[test]
fn reflection_public_constructors_exclude_private_ones() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "publicConstructorsExcludePrivateOnes",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_constructor_get_name_returns_declaring_class_binary_name() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "constructorGetNameReturnsDeclaringClassBinaryName",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_class_new_instance_creates_object_via_public_zero_arg_constructor() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "classNewInstanceCreatesObjectViaPublicZeroArgConstructor",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_class_new_instance_private_zero_arg_raises_illegal_access() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "classNewInstancePrivateZeroArgRaisesIllegalAccess",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_class_new_instance_missing_zero_arg_raises_instantiation() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "classNewInstanceMissingZeroArgRaisesInstantiation",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_field_get_type_returns_primitive_reference_and_array_mirrors() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "fieldGetTypeReturnsPrimitiveReferenceAndArrayMirrors",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_method_get_return_type_returns_primitive_reference_and_array_mirrors() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "methodGetReturnTypeReturnsPrimitiveReferenceAndArrayMirrors",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_method_get_parameter_types_preserve_order_and_kinds() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "methodGetParameterTypesPreserveOrderAndKinds",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_constructor_get_parameter_types_preserve_order_and_kinds() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "constructorGetParameterTypesPreserveOrderAndKinds",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_primitive_wrapper_type_fields_return_expected_names() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "primitiveWrapperTypeFieldsReturnExpectedNames",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_primitive_class_literals_agree_with_wrapper_type_fields() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "primitiveClassLiteralsAgreeWithWrapperTypeFields",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_boolean_wrapper_round_trip() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "booleanWrapperRoundTrip", "()I"),
        1
    );
}

#[test]
fn reflection_float_wrapper_round_trip() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "floatWrapperRoundTrip", "()I"),
        1
    );
}

#[test]
fn reflection_byte_wrapper_round_trip() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "byteWrapperRoundTrip", "()I"),
        1
    );
}

#[test]
fn reflection_short_wrapper_round_trip() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "shortWrapperRoundTrip", "()I"),
        1
    );
}

#[test]
fn reflection_boolean_wrapper_compare_to_orders_false_before_true() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "booleanWrapperCompareToOrdersFalseBeforeTrue",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_float_wrapper_compare_to_orders_values() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "floatWrapperCompareToOrdersValues",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_byte_wrapper_compare_to_orders_values() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "byteWrapperCompareToOrdersValues",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_short_wrapper_compare_to_orders_values() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "shortWrapperCompareToOrdersValues",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_integer_wrapper_typed_compare_to_orders_values() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "integerWrapperTypedCompareToOrdersValues",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_long_wrapper_typed_compare_to_orders_values() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "longWrapperTypedCompareToOrdersValues",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_double_wrapper_typed_compare_to_orders_values() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "doubleWrapperTypedCompareToOrdersValues",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_float_wrapper_typed_compare_to_orders_values() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "floatWrapperTypedCompareToOrdersValues",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_boolean_wrapper_typed_compare_to_orders_false_before_true() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "booleanWrapperTypedCompareToOrdersFalseBeforeTrue",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_byte_wrapper_typed_compare_to_orders_values() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "byteWrapperTypedCompareToOrdersValues",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_short_wrapper_typed_compare_to_orders_values() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "shortWrapperTypedCompareToOrdersValues",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_character_wrapper_typed_compare_to_orders_values() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "characterWrapperTypedCompareToOrdersValues",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_byte_parse_round_trip() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "byteParseRoundTrip", "()I"),
        1
    );
}

#[test]
fn reflection_short_parse_round_trip() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "shortParseRoundTrip", "()I"),
        1
    );
}

#[test]
fn reflection_byte_parse_rejects_out_of_range() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "byteParseRejectsOutOfRange", "()I"),
        1
    );
}

#[test]
fn reflection_short_parse_rejects_out_of_range() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "shortParseRejectsOutOfRange", "()I"),
        1
    );
}

#[test]
fn reflection_byte_value_of_string_round_trip() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "byteValueOfStringRoundTrip", "()I"),
        1
    );
}

#[test]
fn reflection_short_value_of_string_round_trip() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "shortValueOfStringRoundTrip", "()I"),
        1
    );
}

#[test]
fn reflection_byte_value_of_string_rejects_out_of_range() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "byteValueOfStringRejectsOutOfRange",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_short_value_of_string_rejects_out_of_range() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "shortValueOfStringRejectsOutOfRange",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_byte_parse_with_radix_round_trip() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "byteParseWithRadixRoundTrip", "()I"),
        1
    );
}

#[test]
fn reflection_short_parse_with_radix_round_trip() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "shortParseWithRadixRoundTrip",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_byte_value_of_string_with_radix_round_trip() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "byteValueOfStringWithRadixRoundTrip",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_short_value_of_string_with_radix_round_trip() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "shortValueOfStringWithRadixRoundTrip",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_byte_parse_with_invalid_radix_rejects() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "byteParseWithInvalidRadixRejects",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_short_value_of_string_with_radix_rejects_out_of_range() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "shortValueOfStringWithRadixRejectsOutOfRange",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_integer_parse_with_radix_round_trip() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "integerParseWithRadixRoundTrip",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_long_parse_with_radix_round_trip() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "longParseWithRadixRoundTrip", "()I"),
        1
    );
}

#[test]
fn reflection_integer_value_of_string_round_trip() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "integerValueOfStringRoundTrip",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_long_value_of_string_round_trip() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "longValueOfStringRoundTrip", "()I"),
        1
    );
}

#[test]
fn reflection_integer_value_of_string_with_radix_round_trip() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "integerValueOfStringWithRadixRoundTrip",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_long_value_of_string_with_radix_round_trip() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "longValueOfStringWithRadixRoundTrip",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_integer_parse_with_invalid_radix_rejects() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "integerParseWithInvalidRadixRejects",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_long_value_of_string_with_radix_rejects_out_of_range() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "longValueOfStringWithRadixRejectsOutOfRange",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_integer_decode_hex_prefix_round_trip() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "integerDecodeHexPrefixRoundTrip",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_integer_decode_octal_prefix_round_trip() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "integerDecodeOctalPrefixRoundTrip",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_integer_decode_negative_min_hex_round_trip() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "integerDecodeNegativeMinHexRoundTrip",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_integer_decode_rejects_sign_after_prefix() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "integerDecodeRejectsSignAfterPrefix",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_long_decode_hash_prefix_round_trip() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "longDecodeHashPrefixRoundTrip",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_long_decode_leading_plus_hex_round_trip() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "longDecodeLeadingPlusHexRoundTrip",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_long_decode_negative_min_hex_round_trip() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "longDecodeNegativeMinHexRoundTrip",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_long_decode_rejects_positive_overflow() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "longDecodeRejectsPositiveOverflow",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_integer_to_hex_string_positive() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "integerToHexStringPositive", "()I"),
        1
    );
}

#[test]
fn reflection_integer_to_octal_string_positive() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "integerToOctalStringPositive",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_integer_to_binary_string_positive() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "integerToBinaryStringPositive",
            "()I"
        ),
        1
    );
}

#[test]
fn reflection_integer_to_hex_string_negative_uses_unsigned_bits() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "integerToHexStringNegativeUsesUnsignedBits",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_long_to_hex_string_positive() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "longToHexStringPositive", "()I"),
        1
    );
}

#[test]
fn reflection_long_to_octal_string_positive() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "longToOctalStringPositive", "()I"),
        1
    );
}

#[test]
fn reflection_long_to_binary_string_positive() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "longToBinaryStringPositive", "()I"),
        1
    );
}

#[test]
fn reflection_long_to_hex_string_negative_uses_unsigned_bits() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "longToHexStringNegativeUsesUnsignedBits",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_integer_to_unsigned_long_extends_negative_int() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "integerToUnsignedLongExtendsNegativeInt",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_integer_compare_unsigned_orders_negative_as_larger() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "integerCompareUnsignedOrdersNegativeAsLarger",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_long_compare_unsigned_orders_negative_as_larger() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "longCompareUnsignedOrdersNegativeAsLarger",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_long_compare_unsigned_treats_min_as_positive_half_range() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "longCompareUnsignedTreatsMinAsPositiveHalfRange",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_static_field_get_triggers_class_initialization() {
    assert_eq!(
        run_bootstrap_int(
            "ReflectionTest.class",
            "staticFieldGetTriggersInitialization",
            "()I",
        ),
        1
    );
}

#[test]
fn reflection_static_field_set_writes_back() {
    assert_eq!(
        run_bootstrap_int("ReflectionTest.class", "staticFieldSetWritesBack", "()I"),
        1
    );
}

// ---- Phase 32: Process management fixture coverage ----

fn repo_root_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn host_process_command() -> String {
    if cfg!(windows) {
        "powershell".to_string()
    } else {
        "sh".to_string()
    }
}

#[test]
fn process_spawn_and_read_stdout_returns_expected_sum() {
    let result = run_bootstrap_with_string_args(
        "ProcessManagementTest.class",
        "spawnAndReadStdout",
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
        &[
            host_process_command(),
            fixtures_dir().to_string_lossy().into_owned(),
            repo_root_dir().to_string_lossy().into_owned(),
        ],
    );

    assert!(
        result.is_ok(),
        "expected ProcessBuilder.start stdout support; current Duke failed with {result:?}"
    );
    assert_eq!(result.unwrap(), Some(Slot::Int(1)));
}

#[test]
fn process_runtime_exec_reads_stdout_and_exit_code() {
    let result = run_bootstrap_with_string_args(
        "ProcessManagementTest.class",
        "runtimeExecReadsStdout",
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
        &[
            host_process_command(),
            fixtures_dir().to_string_lossy().into_owned(),
            repo_root_dir().to_string_lossy().into_owned(),
        ],
    );

    assert!(
        result.is_ok(),
        "expected Runtime.exec stdout support; current Duke failed with {result:?}"
    );
    assert_eq!(result.unwrap(), Some(Slot::Int(1)));
}

#[test]
fn process_builder_applies_working_directory() {
    let result = run_bootstrap_with_string_args(
        "ProcessManagementTest.class",
        "processBuilderAppliesWorkingDirectory",
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
        &[
            host_process_command(),
            fixtures_dir().to_string_lossy().into_owned(),
            repo_root_dir().to_string_lossy().into_owned(),
        ],
    );

    assert!(
        result.is_ok(),
        "expected ProcessBuilder.directory working directory support; current Duke failed with {result:?}"
    );
    assert_eq!(result.unwrap(), Some(Slot::Int(1)));
}

#[test]
fn process_runtime_exec_applies_working_directory() {
    let result = run_bootstrap_with_string_args(
        "ProcessManagementTest.class",
        "runtimeExecAppliesWorkingDirectory",
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
        &[
            host_process_command(),
            fixtures_dir().to_string_lossy().into_owned(),
            repo_root_dir().to_string_lossy().into_owned(),
        ],
    );

    assert!(
        result.is_ok(),
        "expected Runtime.exec working directory support; current Duke failed with {result:?}"
    );
    assert_eq!(result.unwrap(), Some(Slot::Int(1)));
}

#[test]
fn process_pipe_stdin_to_child_round_trips() {
    let result = run_bootstrap_with_string_args(
        "ProcessManagementTest.class",
        "pipeStdinToChild",
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
        &[
            host_process_command(),
            fixtures_dir().to_string_lossy().into_owned(),
            repo_root_dir().to_string_lossy().into_owned(),
        ],
    );

    assert!(
        result.is_ok(),
        "expected child stdin/stdout pipe support; current Duke failed with {result:?}"
    );
    assert_eq!(result.unwrap(), Some(Slot::Int(1)));
}

#[test]
fn process_read_error_stream_returns_expected_sum() {
    let result = run_bootstrap_with_string_args(
        "ProcessManagementTest.class",
        "readErrorStream",
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
        &[
            host_process_command(),
            fixtures_dir().to_string_lossy().into_owned(),
            repo_root_dir().to_string_lossy().into_owned(),
        ],
    );

    assert!(
        result.is_ok(),
        "expected child stderr pipe support; current Duke failed with {result:?}"
    );
    assert_eq!(result.unwrap(), Some(Slot::Int(1)));
}

#[test]
fn process_destroy_terminates_sleeping_child() {
    let result = run_bootstrap_with_string_args(
        "ProcessManagementTest.class",
        "destroySleepingChild",
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
        &[
            host_process_command(),
            fixtures_dir().to_string_lossy().into_owned(),
            repo_root_dir().to_string_lossy().into_owned(),
        ],
    );

    assert!(
        result.is_ok(),
        "expected Process.destroy and exitValue support; current Duke failed with {result:?}"
    );
    assert_eq!(result.unwrap(), Some(Slot::Int(1)));
}

#[test]
fn process_missing_executable_raises_io_exception() {
    let result = run_bootstrap_int(
        "ProcessManagementTest.class",
        "missingExecutableRaisesIoException",
        "()I",
    );
    assert_eq!(result, 1);
}

// ---- Phase 33: Time primitive fixture coverage ----

#[test]
fn time_current_time_millis_matches_host_wall_clock_window() {
    let host_before = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("host clock is after epoch")
            .as_millis(),
    )
    .expect("millis fit in i64");
    let value = run_bootstrap_long("TimePrimitivesTest.class", "currentTimeMillisNow", "()J");
    let host_after = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("host clock is after epoch")
            .as_millis(),
    )
    .expect("millis fit in i64");

    assert!(
        value >= host_before && value <= host_after,
        "expected {value} within [{host_before}, {host_after}]",
    );
}

#[test]
fn time_current_time_millis_advances_after_sleep() {
    assert_eq!(
        run_bootstrap_int_completion(
            "TimePrimitivesTest.class",
            "currentTimeMillisAdvancesAfterSleep",
            "()I",
        ),
        1,
    );
}

#[test]
fn time_nano_time_returns_positive_elapsed_duration() {
    let delta =
        run_bootstrap_long("TimePrimitivesTest.class", "nanoTimeDeltaAfterSleep", "()J");
    assert!(
        delta >= 1_000_000,
        "expected at least 1 ms in nanos, got {delta}"
    );
}

#[test]
fn time_nano_time_supports_java_duration_math() {
    assert_eq!(
        run_bootstrap_int_completion(
            "TimePrimitivesTest.class",
            "nanoTimeSupportsDurationMath",
            "()I",
        ),
        1,
    );
}

#[test]
fn time_system_time_to_epoch_millis_converts_forward_values() {
    let sample = std::time::UNIX_EPOCH + std::time::Duration::from_millis(1_234);
    assert_eq!(system_time_to_epoch_millis(sample), 1_234);
}

#[test]
fn time_system_time_to_epoch_millis_clamps_pre_epoch_to_zero() {
    let sample = std::time::UNIX_EPOCH - std::time::Duration::from_secs(1);
    assert_eq!(system_time_to_epoch_millis(sample), 0);
}

#[test]
fn time_monotonic_nano_time_is_non_decreasing() {
    let first = monotonic_nano_time_now();
    std::thread::sleep(std::time::Duration::from_millis(1));
    let second = monotonic_nano_time_now();
    assert!(
        second >= first,
        "expected non-decreasing nanos: {first} -> {second}"
    );
}

// ---- Phase 29: Networking helpers and integration tests ----

fn run_bootstrap_with_slots(
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<Option<Slot>> {
    let ctx = load_class_context(class_name);
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let mut out: Vec<u8> = Vec::new();
    execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut out,
        &entry_class,
        method_name,
        descriptor,
        args,
    )
}

#[test]
fn net_bind_and_get_port() {
    let port = run_bootstrap_int("NetworkingTest.class", "bindAndGetPort", "()I");
    assert!(port > 0, "expected positive port, got {port}");
}

#[test]
fn net_connect_refused() {
    // Bind then immediately drop so nothing listens on that port
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    // Brief sleep ensures the OS releases the port before Java tries to connect.
    // No TIME_WAIT applies (listener was never accepted-on), but the sleep guards
    // against any OS-specific teardown delay.
    std::thread::sleep(std::time::Duration::from_millis(10));
    let result = run_bootstrap_with_slots(
        "NetworkingTest.class",
        "connectRefused",
        "(I)I",
        &[Slot::Int(i32::from(port))],
    )
    .expect("connectRefused failed");
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn net_accept_and_read() {
    use std::io::Read;
    // Rust is the server — no drop, no rebind race
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 1];
        stream.read_exact(&mut buf).unwrap();
        assert_eq!(buf[0], 42);
    });
    // Java is the client, writes byte 42
    let result = run_bootstrap_with_slots(
        "NetworkingTest.class",
        "connectAndWriteByte",
        "(II)I",
        &[Slot::Int(i32::from(port)), Slot::Int(42)],
    )
    .expect("connectAndWriteByte failed");
    handle.join().unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn net_connect_and_write() {
    // Rust side: listen; Java side: connect and write byte 99
    use std::io::Read;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 1];
        stream.read_exact(&mut buf).unwrap();
        assert_eq!(buf[0], 99);
    });

    let result = run_bootstrap_with_slots(
        "NetworkingTest.class",
        "connectAndWriteByte",
        "(II)I",
        &[Slot::Int(i32::from(port)), Slot::Int(99)],
    )
    .expect("connectAndWriteByte failed");
    handle.join().unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn net_echo_roundtrip() {
    use std::io::{Read, Write};
    // Rust is the echo server — keeps listener alive, no race
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        // Echo each byte individually so Java's write-then-read loop doesn't deadlock
        let mut buf = [0u8; 1];
        for _ in 0..3 {
            stream.read_exact(&mut buf).unwrap();
            stream.write_all(&buf).unwrap();
        }
    });
    // Java is the client — connects, writes [1,2,3], reads them back
    let result = run_bootstrap_with_slots(
        "NetworkingTest.class",
        "connectAndEchoCheck",
        "(II)I",
        &[Slot::Int(i32::from(port)), Slot::Int(3)],
    )
    .expect("connectAndEchoCheck failed");
    handle.join().unwrap();
    assert_eq!(result, Some(Slot::Int(3)));
}

// ── ZIP / JAR tests ─────────────────────────────────────────────────

fn fixtures_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
}

#[allow(clippy::cast_possible_truncation)]
fn test_zip_crc32(data: &[u8]) -> u32 {
    const TABLE: [u32; 256] = {
        let mut table = [0_u32; 256];
        let mut i: usize = 0;
        while i < 256 {
            let mut crc = i as u32;
            let mut j = 0;
            while j < 8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xEDB8_8320;
                } else {
                    crc >>= 1;
                }
                j += 1;
            }
            table[i] = crc;
            i += 1;
        }
        table
    };

    let mut crc = 0xFFFF_FFFF_u32;
    for &byte in data {
        let idx = ((crc ^ u32::from(byte)) & 0xFF) as usize;
        crc = (crc >> 8) ^ TABLE[idx];
    }
    !crc
}

#[allow(clippy::cast_possible_truncation)]
fn build_test_multi_entry_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    const LOCAL_SIGNATURE: u32 = 0x0403_4b50;
    const CD_SIGNATURE: u32 = 0x0201_4b50;
    const EOCD_SIGNATURE: u32 = 0x0605_4b50;
    const METHOD_STORED: u16 = 0;

    let count = entries.len() as u16;
    let mut zip = Vec::new();
    let mut local_offsets = Vec::with_capacity(entries.len());

    for &(name, content) in entries {
        let crc = test_zip_crc32(content);
        let size = content.len() as u32;
        let name_bytes = name.as_bytes();

        local_offsets.push(zip.len() as u32);
        zip.extend_from_slice(&LOCAL_SIGNATURE.to_le_bytes());
        zip.extend_from_slice(&20_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&METHOD_STORED.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&crc.to_le_bytes());
        zip.extend_from_slice(&size.to_le_bytes());
        zip.extend_from_slice(&size.to_le_bytes());
        zip.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(name_bytes);
        zip.extend_from_slice(content);
    }

    let cd_offset = zip.len() as u32;
    for (index, &(name, content)) in entries.iter().enumerate() {
        let crc = test_zip_crc32(content);
        let size = content.len() as u32;
        let name_bytes = name.as_bytes();

        zip.extend_from_slice(&CD_SIGNATURE.to_le_bytes());
        zip.extend_from_slice(&20_u16.to_le_bytes());
        zip.extend_from_slice(&20_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&METHOD_STORED.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&crc.to_le_bytes());
        zip.extend_from_slice(&size.to_le_bytes());
        zip.extend_from_slice(&size.to_le_bytes());
        zip.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u32.to_le_bytes());
        zip.extend_from_slice(&local_offsets[index].to_le_bytes());
        zip.extend_from_slice(name_bytes);
    }
    let cd_size = (zip.len() as u32) - cd_offset;

    zip.extend_from_slice(&EOCD_SIGNATURE.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&count.to_le_bytes());
    zip.extend_from_slice(&count.to_le_bytes());
    zip.extend_from_slice(&cd_size.to_le_bytes());
    zip.extend_from_slice(&cd_offset.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());

    zip
}

fn build_test_boot_archive_jar(
    start_class: &str,
    archive_entries: &[(&str, Vec<u8>)],
) -> std::path::PathBuf {
    let manifest = format!(
        "Manifest-Version: 1.0\r\nMain-Class: org.springframework.boot.loader.launch.JarLauncher\r\nStart-Class: {start_class}\r\n\r\n"
    );
    let mut owned_entries: Vec<(String, Vec<u8>)> =
        Vec::with_capacity(archive_entries.len() + 1);
    owned_entries.push(("META-INF/MANIFEST.MF".to_string(), manifest.into_bytes()));
    for (entry_name, entry_bytes) in archive_entries {
        owned_entries.push((entry_name.to_string(), entry_bytes.clone()));
    }

    let entry_refs: Vec<(&str, &[u8])> = owned_entries
        .iter()
        .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
        .collect();
    let jar_bytes = build_test_multi_entry_zip(&entry_refs);
    let jar_path = std::env::temp_dir().join(format!(
        "duke_test_boot_exec_{}.jar",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    std::fs::write(&jar_path, jar_bytes).expect("write boot launcher test jar");
    jar_path
}

fn build_test_launched_loader(
    boot_loader: &duke_loader::ZipLoader,
    registry: &mut ClassRegistry,
    heap: &mut duke_gc::Heap,
    jar_path: &std::path::Path,
) -> u64 {
    registry.set_default_code_source(jar_path.display().to_string());
    registry
        .ensure_loaded(
            "org/springframework/boot/loader/launch/JarLauncher",
            boot_loader,
        )
        .expect("load JarLauncher");

    let launcher_ref = heap.allocate(
        "org/springframework/boot/loader/launch/JarLauncher".to_string(),
        total_instance_field_count(
            registry,
            "org/springframework/boot/loader/launch/JarLauncher",
        ),
    );
    init_object_fields(
        registry,
        heap,
        launcher_ref,
        "org/springframework/boot/loader/launch/JarLauncher",
    );
    execute_class(
        registry,
        boot_loader,
        heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/JarLauncher",
        "<init>",
        "()V",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("JarLauncher.<init>() should succeed");
    let urls = execute_class(
        registry,
        boot_loader,
        heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "getClassPathUrls",
        "()Ljava/util/Set;",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("getClassPathUrls should succeed")
    .expect("getClassPathUrls should return a set");
    let Slot::Reference(Some(urls_ref)) = urls else {
        panic!("expected class path set");
    };
    let launched_loader = execute_class(
        registry,
        boot_loader,
        heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "createClassLoader",
        "(Ljava/util/Collection;)Ljava/lang/ClassLoader;",
        &[
            Slot::Reference(Some(launcher_ref)),
            Slot::Reference(Some(urls_ref)),
        ],
    )
    .expect("createClassLoader(Collection) should succeed")
    .expect("createClassLoader(Collection) should return a loader");
    let Slot::Reference(Some(launched_loader_ref)) = launched_loader else {
        panic!("expected launched class loader reference");
    };
    launched_loader_ref
}

fn load_class_via_launched_loader(
    boot_loader: &duke_loader::ZipLoader,
    registry: &mut ClassRegistry,
    heap: &mut duke_gc::Heap,
    binary_name: &str,
    launched_loader_ref: u64,
) -> u64 {
    let binary_name_ref = heap.allocate_string(binary_name.to_string());
    let mut ops = InterpreterCallbackOps {
        registry,
        loader: boot_loader,
    };
    let class_slot = native_class_for_name_with_loader(
        &[
            Slot::Reference(Some(binary_name_ref)),
            Slot::Int(0),
            Slot::Reference(Some(launched_loader_ref)),
        ],
        heap,
        &mut Vec::new(),
        &mut NativeControl::default(),
        &mut ops,
    )
    .expect("Class.forName should succeed")
    .expect("Class.forName should return a class");
    let Slot::Reference(Some(class_ref)) = class_slot else {
        panic!("expected Class reference");
    };
    class_ref
}

fn load_duplicate_hello_world_classes() -> (
    duke_loader::ZipLoader,
    ClassRegistry,
    duke_gc::Heap,
    std::path::PathBuf,
    std::path::PathBuf,
    String,
    String,
) {
    let boot_loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let hello_world = std::fs::read(fixtures_dir().join("HelloWorld.class"))
        .expect("read HelloWorld.class fixture");
    let jar_path_one = build_test_boot_archive_jar(
        "HelloWorld",
        &[("BOOT-INF/classes/HelloWorld.class", hello_world.clone())],
    );
    let jar_path_two = build_test_boot_archive_jar(
        "HelloWorld",
        &[("BOOT-INF/classes/HelloWorld.class", hello_world)],
    );

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let launched_loader_one =
        build_test_launched_loader(&boot_loader, &mut registry, &mut heap, &jar_path_one);
    let launched_loader_two =
        build_test_launched_loader(&boot_loader, &mut registry, &mut heap, &jar_path_two);
    let class_one_ref = load_class_via_launched_loader(
        &boot_loader,
        &mut registry,
        &mut heap,
        "HelloWorld",
        launched_loader_one,
    );
    let class_two_ref = load_class_via_launched_loader(
        &boot_loader,
        &mut registry,
        &mut heap,
        "HelloWorld",
        launched_loader_two,
    );
    let class_key_one = class_key_from_ref(&heap, class_one_ref)
        .expect("loader-one mirror should carry class key");
    let class_key_two = class_key_from_ref(&heap, class_two_ref)
        .expect("loader-two mirror should carry class key");

    (
        boot_loader,
        registry,
        heap,
        jar_path_one,
        jar_path_two,
        class_key_one,
        class_key_two,
    )
}

fn install_loader_keyed_hello_world_probe(
    registry: &mut ClassRegistry,
    class_key: &str,
    method_name: &str,
    descriptor: &str,
    instructions: Vec<(usize, Instruction)>,
    exception_table: Vec<ExceptionEntry>,
) {
    use duke_classfile::types::CpIndex;
    use std::sync::Arc;

    let pc_to_idx: std::collections::HashMap<usize, usize> = instructions
        .iter()
        .enumerate()
        .map(|(idx, (pc, _))| (*pc, idx))
        .collect();
    let method = MethodEntry {
        name: method_name.to_string(),
        descriptor: descriptor.to_string(),
        is_public: true,
        is_static: true,
        instructions: instructions.into(),
        max_stack: 2,
        max_locals: 1,
        exception_table,
        pc_to_idx: Arc::new(pc_to_idx),
    };
    let ctx = registry
        .get_mut(class_key)
        .expect("probe class should already be loaded");
    ctx.constant_pool = vec![
        None,
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("HelloWorld".to_string())),
    ];
    ctx.methods.push(method);
}

fn allocate_zero_field_object(
    registry: &ClassRegistry,
    heap: &mut duke_gc::Heap,
    class_key: &str,
) -> u64 {
    let object_ref = heap.allocate(
        class_key.to_string(),
        total_instance_field_count(registry, class_key),
    );
    init_object_fields(registry, heap, object_ref, class_key);
    object_ref
}

fn invoke_class_get_class_loader(
    boot_loader: &duke_loader::ZipLoader,
    registry: &mut ClassRegistry,
    heap: &mut duke_gc::Heap,
    class_ref: u64,
) -> Slot {
    match registry.natives_mut().get_kind(
        "java/lang/Class",
        "getClassLoader",
        "()Ljava/lang/ClassLoader;",
    ) {
        Some(HandlerKind::Simple(handler)) => handler(
            &[Slot::Reference(Some(class_ref))],
            heap,
            &mut Vec::new(),
            &mut NativeControl::default(),
        ),
        Some(HandlerKind::Callback(handler)) => {
            let mut ops = InterpreterCallbackOps {
                registry,
                loader: boot_loader,
            };
            handler(
                &[Slot::Reference(Some(class_ref))],
                heap,
                &mut Vec::new(),
                &mut NativeControl::default(),
                &mut ops,
            )
        }
        other => panic!("expected Class.getClassLoader native, got {other:?}"),
    }
    .expect("Class.getClassLoader should succeed")
    .expect("Class.getClassLoader should return a ClassLoader")
}

#[test]
fn zip_loader_loads_class_from_jar() {
    let jar_path = fixtures_dir().join("hello.jar");
    let loader = duke_loader::ZipLoader::open(&jar_path).expect("should open hello.jar");
    let bytes = loader
        .find_class("HelloWorld")
        .expect("should find HelloWorld in JAR");
    assert_eq!(&bytes[..4], &[0xCA, 0xFE, 0xBA, 0xBE]);
}

#[test]
fn execute_class_loaded_from_jar() {
    let jar_path = fixtures_dir().join("hello.jar");
    let loader = duke_loader::ZipLoader::open(&jar_path).expect("open hello.jar");
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    // Pre-load the entry class from the JAR.
    registry
        .ensure_loaded("HelloWorld", &loader)
        .expect("load HelloWorld");
    // Build String[] args
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 0);
    let main_args = vec![Slot::Reference(Some(arr_ref))];
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut out,
        "HelloWorld",
        "main",
        "([Ljava/lang/String;)V",
        &main_args,
    );
    assert!(
        result.is_ok(),
        "HelloWorld.main from JAR failed: {}",
        result.unwrap_err()
    );
    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("Hello, World!"), "output was: {output}");
}

#[test]
#[allow(clippy::too_many_lines)]
fn class_protection_domain_chain_resolves_back_to_file() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let jar_path = fixtures_dir().join("hello.jar");
    let jar_path = jar_path.canonicalize().expect("canonical hello.jar");

    let get_pd = match registry.natives_mut().get_kind(
        "java/lang/Class",
        "getProtectionDomain",
        "()Ljava/security/ProtectionDomain;",
    ) {
        Some(HandlerKind::Callback(handler)) => handler,
        other => panic!("expected Class.getProtectionDomain callback, got {other:?}"),
    };
    let get_code_source = match registry.natives_mut().get_kind(
        "java/security/ProtectionDomain",
        "getCodeSource",
        "()Ljava/security/CodeSource;",
    ) {
        Some(HandlerKind::Simple(handler)) => handler,
        other => panic!("expected ProtectionDomain.getCodeSource native, got {other:?}"),
    };
    let get_location = match registry.natives_mut().get_kind(
        "java/security/CodeSource",
        "getLocation",
        "()Ljava/net/URL;",
    ) {
        Some(HandlerKind::Simple(handler)) => handler,
        other => panic!("expected CodeSource.getLocation native, got {other:?}"),
    };
    let url_to_uri =
        match registry
            .natives_mut()
            .get_kind("java/net/URL", "toURI", "()Ljava/net/URI;")
        {
            Some(HandlerKind::Simple(handler)) => handler,
            other => panic!("expected URL.toURI native, got {other:?}"),
        };
    let path_of = match registry.natives_mut().get_kind(
        "java/nio/file/Path",
        "of",
        "(Ljava/net/URI;)Ljava/nio/file/Path;",
    ) {
        Some(HandlerKind::Simple(handler)) => handler,
        other => panic!("expected Path.of native, got {other:?}"),
    };
    let path_to_file = match registry.natives_mut().get_kind(
        "java/nio/file/Path",
        "toFile",
        "()Ljava/io/File;",
    ) {
        Some(HandlerKind::Simple(handler)) => handler,
        other => panic!("expected Path.toFile native, got {other:?}"),
    };

    let class_ref =
        allocate_class_object(&mut heap, "org/springframework/boot/loader/launch/Launcher")
            .expect("allocate Class object");
    let mut ops = FixedCodeSourceOps {
        code_source: Some(jar_path.to_string_lossy().to_string()),
    };
    let mut sink: Vec<u8> = Vec::new();
    let mut control = NativeControl::default();

    let pd_ref = match get_pd(
        &[Slot::Reference(Some(class_ref))],
        &mut heap,
        &mut sink,
        &mut control,
        &mut ops,
    )
    .expect("getProtectionDomain should succeed")
    {
        Some(Slot::Reference(Some(r))) => r,
        other => panic!("expected protection domain reference, got {other:?}"),
    };
    let code_source_ref = match get_code_source(
        &[Slot::Reference(Some(pd_ref))],
        &mut heap,
        &mut sink,
        &mut control,
    )
    .expect("getCodeSource should succeed")
    {
        Some(Slot::Reference(Some(r))) => r,
        other => panic!("expected code source reference, got {other:?}"),
    };
    let code_source_location_ref = match get_location(
        &[Slot::Reference(Some(code_source_ref))],
        &mut heap,
        &mut sink,
        &mut control,
    )
    .expect("getLocation should succeed")
    {
        Some(Slot::Reference(Some(r))) => r,
        other => panic!("expected URL reference, got {other:?}"),
    };
    let code_source_uri_ref = match url_to_uri(
        &[Slot::Reference(Some(code_source_location_ref))],
        &mut heap,
        &mut sink,
        &mut control,
    )
    .expect("toURI should succeed")
    {
        Some(Slot::Reference(Some(r))) => r,
        other => panic!("expected URI reference, got {other:?}"),
    };
    let path_ref = match path_of(
        &[Slot::Reference(Some(code_source_uri_ref))],
        &mut heap,
        &mut sink,
        &mut control,
    )
    .expect("Path.of should succeed")
    {
        Some(Slot::Reference(Some(r))) => r,
        other => panic!("expected Path reference, got {other:?}"),
    };
    let file_ref = match path_to_file(
        &[Slot::Reference(Some(path_ref))],
        &mut heap,
        &mut sink,
        &mut control,
    )
    .expect("Path.toFile should succeed")
    {
        Some(Slot::Reference(Some(r))) => r,
        other => panic!("expected File reference, got {other:?}"),
    };

    let resolved_path = file_path_from_ref(file_ref, &heap).expect("file path from ref");
    let resolved_canonical = resolved_path
        .canonicalize()
        .expect("canonical resolved path");
    assert_eq!(resolved_canonical, jar_path);
}

#[test]
fn paths_get_joins_first_and_more_segments() {
    let mut heap = duke_gc::Heap::new();
    let first_ref = heap.allocate_string("boot".to_string());
    let lib_ref = heap.allocate_string("lib".to_string());
    let jar_ref = heap.allocate_string("app.jar".to_string());
    let more_ref = heap.allocate("[Ljava/lang/String;".to_string(), 2);
    heap.get_mut(more_ref).unwrap().fields = vec![
        Slot::Reference(Some(lib_ref)),
        Slot::Reference(Some(jar_ref)),
    ];
    let mut sink: Vec<u8> = Vec::new();
    let result = native_paths_get(
        &[
            Slot::Reference(Some(first_ref)),
            Slot::Reference(Some(more_ref)),
        ],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .unwrap();

    let Slot::Reference(Some(path_ref)) = result.unwrap() else {
        panic!("expected Path reference");
    };
    let expected = std::path::PathBuf::from("boot")
        .join("lib")
        .join("app.jar")
        .to_string_lossy()
        .into_owned();
    assert_eq!(
        string_backed_object_value(&heap, path_ref).unwrap(),
        expected
    );
}

#[test]
fn native_set_of_builds_hashset_from_array_elements() {
    let mut heap = duke_gc::Heap::new();
    let alpha_ref = heap.allocate_string("alpha".to_string());
    let beta_ref = heap.allocate_string("beta".to_string());
    let array_ref = heap.allocate("[Ljava/lang/Object;".to_string(), 2);
    heap.get_mut(array_ref).unwrap().fields = vec![
        Slot::Reference(Some(alpha_ref)),
        Slot::Reference(Some(beta_ref)),
    ];
    let mut sink: Vec<u8> = Vec::new();

    let result = native_set_of(
        &[Slot::Reference(Some(array_ref))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .expect("Set.of should succeed")
    .expect("Set.of should return a set");

    let Slot::Reference(Some(set_ref)) = result else {
        panic!("expected HashSet reference");
    };
    let set = heap.get(set_ref).unwrap();
    assert_eq!(set.class_name, "java/util/HashSet");
    assert_eq!(set.fields.first(), Some(&Slot::Int(2)));
    assert_eq!(set.fields.len(), 3);
}

#[test]
fn bootstrap_stdlib_registers_posix_permission_singletons() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let ctx = registry
        .get("java/nio/file/attribute/PosixFilePermission")
        .expect("PosixFilePermission should be registered");
    assert_eq!(ctx.static_fields.len(), 3);
    for slot in &ctx.static_fields {
        assert!(matches!(slot, Slot::Reference(Some(_))));
    }
}

#[test]
fn native_posix_file_permissions_as_file_attribute_returns_placeholder() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();

    let result = native_posix_file_permissions_as_file_attribute(
        &[Slot::Reference(None)],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .expect("asFileAttribute should succeed")
    .expect("asFileAttribute should return a placeholder");

    let Slot::Reference(Some(attribute_ref)) = result else {
        panic!("expected FileAttribute reference");
    };
    assert_eq!(
        heap.get(attribute_ref).unwrap().class_name,
        "java/nio/file/attribute/FileAttribute"
    );
}

#[test]
fn bootstrap_stdlib_registers_object_get_class_native() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let get_class = match registry.natives_mut().get_kind(
        "java/lang/Object",
        "getClass",
        "()Ljava/lang/Class;",
    ) {
        Some(HandlerKind::Simple(handler)) => handler,
        other => panic!("expected Object.getClass native, got {other:?}"),
    };

    let object_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    let mut sink: Vec<u8> = Vec::new();
    let result = get_class(
        &[Slot::Reference(Some(object_ref))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .expect("Object.getClass should succeed")
    .expect("Object.getClass should return a Class object");

    let Slot::Reference(Some(class_ref)) = result else {
        panic!("expected Class reference");
    };
    assert_eq!(
        class_internal_name_from_ref(&heap, class_ref).unwrap(),
        "java/util/HashSet"
    );
}

#[test]
fn bootstrap_stdlib_registers_collection_to_array_natives() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let to_array = match registry.natives_mut().get_kind(
        "java/util/Collection",
        "toArray",
        "()[Ljava/lang/Object;",
    ) {
        Some(HandlerKind::Simple(handler)) => handler,
        other => panic!("expected Collection.toArray() native, got {other:?}"),
    };
    let to_array_with_seed = match registry.natives_mut().get_kind(
        "java/util/Collection",
        "toArray",
        "([Ljava/lang/Object;)[Ljava/lang/Object;",
    ) {
        Some(HandlerKind::Simple(handler)) => handler,
        other => panic!("expected Collection.toArray(Object[]) native, got {other:?}"),
    };

    let collection_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    let mut sink: Vec<u8> = Vec::new();
    native_hashset_init(
        &[Slot::Reference(Some(collection_ref))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .expect("HashSet init should succeed");

    let first_url = heap.allocate("java/net/URL".to_string(), 1);
    let second_url = heap.allocate("java/net/URL".to_string(), 1);
    native_hashset_add(
        &[
            Slot::Reference(Some(collection_ref)),
            Slot::Reference(Some(first_url)),
        ],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .expect("HashSet.add should succeed");
    native_hashset_add(
        &[
            Slot::Reference(Some(collection_ref)),
            Slot::Reference(Some(second_url)),
        ],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .expect("HashSet.add should succeed");

    let array_result = to_array(
        &[Slot::Reference(Some(collection_ref))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .expect("Collection.toArray() should succeed")
    .expect("Collection.toArray() should return an array");
    let Slot::Reference(Some(array_ref)) = array_result else {
        panic!("expected Object[] reference");
    };
    let object_array = heap.get(array_ref).unwrap();
    assert_eq!(object_array.class_name, "[Ljava/lang/Object;");
    assert_eq!(object_array.fields.len(), 2);
    assert_eq!(object_array.fields[0], Slot::Reference(Some(first_url)));
    assert_eq!(object_array.fields[1], Slot::Reference(Some(second_url)));

    let seed_array_ref = heap.allocate("[Ljava/net/URL;".to_string(), 0);
    let typed_result = to_array_with_seed(
        &[
            Slot::Reference(Some(collection_ref)),
            Slot::Reference(Some(seed_array_ref)),
        ],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .expect("Collection.toArray(Object[]) should succeed")
    .expect("Collection.toArray(Object[]) should return an array");
    let Slot::Reference(Some(typed_ref)) = typed_result else {
        panic!("expected typed array reference");
    };
    let typed_array = heap.get(typed_ref).unwrap();
    assert_eq!(typed_array.class_name, "[Ljava/net/URL;");
    assert_eq!(typed_array.fields.len(), 2);
    assert_eq!(typed_array.fields[0], Slot::Reference(Some(first_url)));
    assert_eq!(typed_array.fields[1], Slot::Reference(Some(second_url)));
}

#[test]
fn bootstrap_stdlib_replaces_boot_launcher_shims_with_archive_and_ctor_shims() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    assert!(
        registry
            .natives_mut()
            .get_kind(
                "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
                "getClassPathUrls",
                "()Ljava/util/Set;",
            )
            .is_none(),
        "getClassPathUrls() should use launcher bytecode, not a native shim"
    );
    assert!(
        registry
            .natives_mut()
            .get_kind(
                "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
                "createClassLoader",
                "(Ljava/util/Collection;)Ljava/lang/ClassLoader;",
            )
            .is_none(),
        "createClassLoader(Collection) should use bytecode, not a native shim"
    );
    assert!(
        matches!(
            registry.natives_mut().get_kind(
                "org/springframework/boot/loader/launch/Archive",
                "getManifest",
                "()Ljava/util/jar/Manifest;",
            ),
            Some(HandlerKind::Simple(_))
        ),
        "expected Archive.getManifest shim to be registered"
    );
    assert!(
        matches!(
            registry.natives_mut().get_kind(
                "org/springframework/boot/loader/launch/JarFileArchive",
                "getManifest",
                "()Ljava/util/jar/Manifest;",
            ),
            Some(HandlerKind::Simple(_))
        ),
        "expected JarFileArchive.getManifest shim to be registered"
    );
    assert!(
        matches!(
            registry.natives_mut().get_kind(
                "org/springframework/boot/loader/launch/ExplodedArchive",
                "getManifest",
                "()Ljava/util/jar/Manifest;",
            ),
            Some(HandlerKind::Simple(_))
        ),
        "expected ExplodedArchive.getManifest shim to be registered"
    );
    assert!(
        matches!(
            registry.natives_mut().get_kind(
                "org/springframework/boot/loader/launch/JarFileArchive",
                "getClassPathUrls",
                "(Ljava/util/function/Predicate;Ljava/util/function/Predicate;)Ljava/util/Set;",
            ),
            Some(HandlerKind::Callback(_))
        ),
        "expected JarFileArchive.getClassPathUrls shim to be registered"
    );
    assert!(
        matches!(
            registry.natives_mut().get_kind(
                "org/springframework/boot/loader/launch/ExplodedArchive",
                "getClassPathUrls",
                "(Ljava/util/function/Predicate;Ljava/util/function/Predicate;)Ljava/util/Set;",
            ),
            Some(HandlerKind::Callback(_))
        ),
        "expected ExplodedArchive.getClassPathUrls shim to be registered"
    );
    assert!(
        matches!(
            registry.natives_mut().get_kind(
                "org/springframework/boot/loader/launch/LaunchedClassLoader",
                "<init>",
                "(ZLorg/springframework/boot/loader/launch/Archive;[Ljava/net/URL;Ljava/lang/ClassLoader;)V",
            ),
            Some(HandlerKind::Callback(_))
        ),
        "expected LaunchedClassLoader ctor shim to be registered"
    );
}

#[test]
fn bootstrap_stdlib_registers_boot_archive_entry_natives() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    assert!(
        matches!(
            registry.natives_mut().get_kind(
                "duke/boot/ArchiveEntry",
                "name",
                "()Ljava/lang/String;",
            ),
            Some(HandlerKind::Simple(_))
        ),
        "expected duke/boot/ArchiveEntry.name native"
    );
    assert!(
        matches!(
            registry
                .natives_mut()
                .get_kind("duke/boot/ArchiveEntry", "isDirectory", "()Z",),
            Some(HandlerKind::Simple(_))
        ),
        "expected duke/boot/ArchiveEntry.isDirectory native"
    );
}

#[test]
fn bootstrap_stdlib_registers_class_loader_parallel_capable_native() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let register_parallel = match registry.natives_mut().get_kind(
        "java/lang/ClassLoader",
        "registerAsParallelCapable",
        "()Z",
    ) {
        Some(HandlerKind::Simple(handler)) => handler,
        other => panic!("expected ClassLoader.registerAsParallelCapable native, got {other:?}"),
    };
    let mut sink: Vec<u8> = Vec::new();
    let result = register_parallel(&[], &mut heap, &mut sink, &mut NativeControl::default())
        .expect("registerAsParallelCapable should succeed")
        .expect("registerAsParallelCapable should return a boolean");
    assert_eq!(result, Slot::Int(1));
}

#[test]
fn native_boot_jar_file_archive_get_class_path_urls_uses_include_predicate() {
    struct ArchivePredicateOps;

    impl CallbackOps for ArchivePredicateOps {
        fn invoke(
            &mut self,
            heap: &mut duke_gc::Heap,
            _output: &mut dyn Write,
            class: &str,
            method: &str,
            descriptor: &str,
            args: Vec<Slot>,
        ) -> VmResult<Option<Slot>> {
            assert_eq!(method, "test");
            assert_eq!(descriptor, "(Ljava/lang/Object;)Z");
            let entry_ref = match args.get(1) {
                Some(Slot::Reference(Some(entry_ref))) => *entry_ref,
                other => panic!("expected archive entry arg, got {other:?}"),
            };
            let entry_name = boot_archive_entry_name(heap, entry_ref).unwrap();
            let is_directory = boot_archive_entry_is_directory_flag(heap, entry_ref).unwrap();
            let accepted = match class {
                "duke/test/IncludePredicate" => {
                    (is_directory && entry_name == "BOOT-INF/classes/")
                        || (!is_directory && entry_name == "BOOT-INF/lib/helper.jar")
                }
                "duke/test/SearchPredicate" => false,
                other => panic!("unexpected predicate class {other}"),
            };
            Ok(Some(Slot::Int(i32::from(accepted))))
        }

        fn ensure_loaded(&mut self, _class: &str) -> VmResult<()> {
            Ok(())
        }

        fn inspect_class(&mut self, _class: &str) -> VmResult<ReflectedClassInfo> {
            unreachable!("inspect_class should not be used")
        }
    }

    let jar_bytes = build_test_multi_entry_zip(&[
        ("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\r\n"),
        ("BOOT-INF/classes/", b""),
        ("BOOT-INF/classes/com/example/App.class", b"cafebabe"),
        ("BOOT-INF/lib/helper.jar", b"nested"),
        ("BOOT-INF/lib/ignored.txt", b"ignored"),
    ]);
    let jar_path = std::env::temp_dir().join(format!(
        "duke_test_boot_archive_{}.jar",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    std::fs::write(&jar_path, jar_bytes).expect("write archive test jar");

    let mut heap = duke_gc::Heap::new();
    let file_ref = heap.allocate("java/io/File".to_string(), 1);
    let path_ref = heap.allocate_string(jar_path.to_string_lossy().into_owned());
    heap.get_mut(file_ref).unwrap().fields[0] = Slot::Reference(Some(path_ref));

    let archive_ref = heap.allocate(
        "org/springframework/boot/loader/launch/JarFileArchive".to_string(),
        3,
    );
    heap.get_mut(archive_ref).unwrap().fields[0] = Slot::Reference(Some(file_ref));

    let include_predicate_ref = heap.allocate("duke/test/IncludePredicate".to_string(), 0);
    let search_predicate_ref = heap.allocate("duke/test/SearchPredicate".to_string(), 0);
    let mut sink: Vec<u8> = Vec::new();

    let result = native_boot_jar_file_archive_get_class_path_urls(
        &[
            Slot::Reference(Some(archive_ref)),
            Slot::Reference(Some(include_predicate_ref)),
            Slot::Reference(Some(search_predicate_ref)),
        ],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
        &mut ArchivePredicateOps,
    )
    .expect("JarFileArchive.getClassPathUrls should succeed")
    .expect("JarFileArchive.getClassPathUrls should return a set");

    std::fs::remove_file(&jar_path).ok();

    let Slot::Reference(Some(set_ref)) = result else {
        panic!("expected HashSet reference");
    };
    let set = heap.get(set_ref).unwrap();
    assert_eq!(set.fields.first(), Some(&Slot::Int(2)));

    let urls: Vec<String> = set
        .fields
        .iter()
        .skip(1)
        .filter_map(Slot::as_reference)
        .map(|url_ref| string_backed_object_value(&heap, url_ref).unwrap())
        .collect();
    assert_eq!(urls.len(), 2);
    assert!(urls.iter().any(|url| url.contains("BOOT-INF/classes/")));
    assert!(
        urls.iter()
            .any(|url| url.contains("BOOT-INF/lib/helper.jar"))
    );
}

#[test]
fn native_boot_jar_file_archive_get_manifest_reads_embedded_manifest() {
    let jar_bytes = build_test_multi_entry_zip(&[
        (
            "META-INF/MANIFEST.MF",
            b"Manifest-Version: 1.0\r\nStart-Class: com.example.App\r\n",
        ),
        ("BOOT-INF/classes/com/example/App.class", b"cafebabe"),
    ]);
    let jar_path = std::env::temp_dir().join(format!(
        "duke_test_boot_archive_manifest_{}.jar",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    std::fs::write(&jar_path, jar_bytes).expect("write archive test jar");

    let mut heap = duke_gc::Heap::new();
    let jar_file_ref = heap.allocate("java/util/jar/JarFile".to_string(), 1);
    let file_ref = heap.allocate("java/io/File".to_string(), 1);
    let path_ref = heap.allocate_string(jar_path.to_string_lossy().into_owned());
    heap.get_mut(file_ref).unwrap().fields[0] = Slot::Reference(Some(path_ref));
    native_jar_file_init_from_file(
        &[
            Slot::Reference(Some(jar_file_ref)),
            Slot::Reference(Some(file_ref)),
        ],
        &mut heap,
        &mut Vec::new(),
        &mut NativeControl::default(),
    )
    .expect("JarFile.<init>(File) should succeed");

    let archive_ref = heap.allocate(
        "org/springframework/boot/loader/launch/JarFileArchive".to_string(),
        3,
    );
    heap.get_mut(archive_ref).unwrap().fields[1] = Slot::Reference(Some(jar_file_ref));

    let result = native_boot_jar_file_archive_get_manifest(
        &[Slot::Reference(Some(archive_ref))],
        &mut heap,
        &mut Vec::new(),
        &mut NativeControl::default(),
    )
    .expect("JarFileArchive.getManifest should succeed")
    .expect("JarFileArchive.getManifest should return a manifest");

    std::fs::remove_file(&jar_path).ok();

    let Slot::Reference(Some(manifest_ref)) = result else {
        panic!("expected Manifest reference");
    };
    let manifest_text_ref = heap.get(manifest_ref).unwrap().fields[0]
        .as_reference()
        .expect("Manifest.raw string");
    let manifest_text = string_value_from_ref(&heap, manifest_text_ref).unwrap();
    assert!(manifest_text.contains("Start-Class: com.example.App"));
}

#[test]
fn native_boot_exploded_archive_get_class_path_urls_uses_search_predicate() {
    struct ArchivePredicateOps;

    impl CallbackOps for ArchivePredicateOps {
        fn invoke(
            &mut self,
            heap: &mut duke_gc::Heap,
            _output: &mut dyn Write,
            class: &str,
            method: &str,
            descriptor: &str,
            args: Vec<Slot>,
        ) -> VmResult<Option<Slot>> {
            assert_eq!(method, "test");
            assert_eq!(descriptor, "(Ljava/lang/Object;)Z");
            let entry_ref = match args.get(1) {
                Some(Slot::Reference(Some(entry_ref))) => *entry_ref,
                other => panic!("expected archive entry arg, got {other:?}"),
            };
            let entry_name = boot_archive_entry_name(heap, entry_ref).unwrap();
            let is_directory = boot_archive_entry_is_directory_flag(heap, entry_ref).unwrap();
            let accepted = match class {
                "duke/test/IncludePredicate" => {
                    (is_directory && entry_name == "BOOT-INF/classes/")
                        || (!is_directory && entry_name == "BOOT-INF/lib/helper.jar")
                }
                "duke/test/SearchPredicate" => {
                    is_directory && (entry_name == "BOOT-INF/" || entry_name == "BOOT-INF/lib/")
                }
                other => panic!("unexpected predicate class {other}"),
            };
            Ok(Some(Slot::Int(i32::from(accepted))))
        }

        fn ensure_loaded(&mut self, _class: &str) -> VmResult<()> {
            Ok(())
        }

        fn inspect_class(&mut self, _class: &str) -> VmResult<ReflectedClassInfo> {
            unreachable!("inspect_class should not be used")
        }
    }

    let root_path = std::env::temp_dir().join(format!(
        "duke_test_boot_exploded_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(root_path.join("BOOT-INF/classes/com/example"))
        .expect("create exploded classes dir");
    std::fs::create_dir_all(root_path.join("BOOT-INF/lib")).expect("create exploded lib dir");
    std::fs::write(
        root_path.join("BOOT-INF/classes/com/example/App.class"),
        b"cafebabe",
    )
    .expect("write exploded class");
    std::fs::write(root_path.join("BOOT-INF/lib/helper.jar"), b"nested")
        .expect("write exploded lib");

    let mut heap = duke_gc::Heap::new();
    let file_ref = heap.allocate("java/io/File".to_string(), 1);
    let path_ref = heap.allocate_string(root_path.to_string_lossy().into_owned());
    heap.get_mut(file_ref).unwrap().fields[0] = Slot::Reference(Some(path_ref));

    let archive_ref = heap.allocate(
        "org/springframework/boot/loader/launch/ExplodedArchive".to_string(),
        3,
    );
    heap.get_mut(archive_ref).unwrap().fields[0] = Slot::Reference(Some(file_ref));

    let include_predicate_ref = heap.allocate("duke/test/IncludePredicate".to_string(), 0);
    let search_predicate_ref = heap.allocate("duke/test/SearchPredicate".to_string(), 0);
    let mut sink: Vec<u8> = Vec::new();

    let result = native_boot_exploded_archive_get_class_path_urls(
        &[
            Slot::Reference(Some(archive_ref)),
            Slot::Reference(Some(include_predicate_ref)),
            Slot::Reference(Some(search_predicate_ref)),
        ],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
        &mut ArchivePredicateOps,
    )
    .expect("ExplodedArchive.getClassPathUrls should succeed")
    .expect("ExplodedArchive.getClassPathUrls should return a set");

    std::fs::remove_dir_all(&root_path).ok();

    let Slot::Reference(Some(set_ref)) = result else {
        panic!("expected HashSet reference");
    };
    let set = heap.get(set_ref).unwrap();
    assert_eq!(set.fields.first(), Some(&Slot::Int(2)));

    let urls: Vec<String> = set
        .fields
        .iter()
        .skip(1)
        .filter_map(Slot::as_reference)
        .map(|url_ref| string_backed_object_value(&heap, url_ref).unwrap())
        .collect();
    assert_eq!(urls.len(), 2);
    assert!(urls.iter().any(|url| url.contains("BOOT-INF/classes")));
    assert!(
        urls.iter()
            .any(|url| url.contains("BOOT-INF/lib/helper.jar"))
    );
}

#[test]
fn native_boot_exploded_archive_get_manifest_reads_meta_inf_manifest() {
    let root_path = std::env::temp_dir().join(format!(
        "duke_test_boot_exploded_manifest_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    let manifest_dir = root_path.join("META-INF");
    std::fs::create_dir_all(&manifest_dir).expect("create META-INF");
    std::fs::write(
        manifest_dir.join("MANIFEST.MF"),
        b"Manifest-Version: 1.0\r\nStart-Class: com.example.Exploded\r\n",
    )
    .expect("write manifest");

    let mut heap = duke_gc::Heap::new();
    let file_ref = heap.allocate("java/io/File".to_string(), 1);
    let path_ref = heap.allocate_string(root_path.to_string_lossy().into_owned());
    heap.get_mut(file_ref).unwrap().fields[0] = Slot::Reference(Some(path_ref));
    let archive_ref = heap.allocate(
        "org/springframework/boot/loader/launch/ExplodedArchive".to_string(),
        3,
    );
    heap.get_mut(archive_ref).unwrap().fields[0] = Slot::Reference(Some(file_ref));

    let result = native_boot_exploded_archive_get_manifest(
        &[Slot::Reference(Some(archive_ref))],
        &mut heap,
        &mut Vec::new(),
        &mut NativeControl::default(),
    )
    .expect("ExplodedArchive.getManifest should succeed")
    .expect("ExplodedArchive.getManifest should return a manifest");

    std::fs::remove_dir_all(&root_path).ok();

    let Slot::Reference(Some(manifest_ref)) = result else {
        panic!("expected Manifest reference");
    };
    let manifest_text_ref = heap.get(manifest_ref).unwrap().fields[0]
        .as_reference()
        .expect("Manifest.raw string");
    let manifest_text = string_value_from_ref(&heap, manifest_text_ref).unwrap();
    assert!(manifest_text.contains("Start-Class: com.example.Exploded"));
    assert_eq!(
        heap.get(archive_ref).unwrap().fields[BOOT_EXPLODED_ARCHIVE_MANIFEST_SLOT],
        Slot::Reference(Some(manifest_ref))
    );
}

#[test]
fn native_class_for_name_with_loader_uses_binary_name() {
    #[derive(Default)]
    struct LoadingOps {
        loaded: Vec<String>,
    }

    impl CallbackOps for LoadingOps {
        fn invoke(
            &mut self,
            _heap: &mut duke_gc::Heap,
            _output: &mut dyn Write,
            _class: &str,
            _method: &str,
            _descriptor: &str,
            _args: Vec<Slot>,
        ) -> VmResult<Option<Slot>> {
            Ok(None)
        }

        fn ensure_loaded(&mut self, class: &str) -> VmResult<()> {
            self.loaded.push(class.to_string());
            Ok(())
        }

        fn inspect_class(&mut self, _class: &str) -> VmResult<ReflectedClassInfo> {
            unreachable!("inspect_class should not be used")
        }
    }

    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let binary_name_ref = heap.allocate_string("com.example.BootApp".to_string());
    let mut ops = LoadingOps::default();

    let result = native_class_for_name_with_loader(
        &[
            Slot::Reference(Some(binary_name_ref)),
            Slot::Int(0),
            Slot::Reference(None),
        ],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
        &mut ops,
    )
    .expect("Class.forName overload should succeed")
    .expect("Class.forName overload should return a class");

    let Slot::Reference(Some(class_ref)) = result else {
        panic!("expected Class reference");
    };
    assert_eq!(ops.loaded, vec!["com/example/BootApp".to_string()]);
    assert_eq!(
        class_internal_name_from_ref(&heap, class_ref).unwrap(),
        "com/example/BootApp"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn native_class_for_name_with_loader_uses_loader_archive_not_global_default_code_source() {
    let loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let hello_world = std::fs::read(fixtures_dir().join("HelloWorld.class"))
        .expect("read HelloWorld.class fixture");
    let class_jar_path = build_test_boot_archive_jar(
        "HelloWorld",
        &[("BOOT-INF/classes/HelloWorld.class", hello_world)],
    );
    let empty_jar_path = build_test_boot_archive_jar("Missing", &[]);

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.set_default_code_source(empty_jar_path.display().to_string());
    registry
        .ensure_loaded(
            "org/springframework/boot/loader/launch/JarLauncher",
            &loader,
        )
        .expect("load JarLauncher");
    let launcher_ref = heap.allocate(
        "org/springframework/boot/loader/launch/JarLauncher".to_string(),
        total_instance_field_count(
            &registry,
            "org/springframework/boot/loader/launch/JarLauncher",
        ),
    );
    init_object_fields(
        &registry,
        &mut heap,
        launcher_ref,
        "org/springframework/boot/loader/launch/JarLauncher",
    );
    execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/JarLauncher",
        "<init>",
        "()V",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("JarLauncher.<init>() should succeed");
    let urls = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "getClassPathUrls",
        "()Ljava/util/Set;",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("getClassPathUrls should succeed")
    .expect("getClassPathUrls should return a set");
    let Slot::Reference(Some(urls_ref)) = urls else {
        panic!("expected class path set");
    };
    let launched_loader = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "createClassLoader",
        "(Ljava/util/Collection;)Ljava/lang/ClassLoader;",
        &[
            Slot::Reference(Some(launcher_ref)),
            Slot::Reference(Some(urls_ref)),
        ],
    )
    .expect("createClassLoader(Collection) should succeed")
    .expect("createClassLoader(Collection) should return a loader");
    let Slot::Reference(Some(launched_loader_ref)) = launched_loader else {
        panic!("expected launched class loader reference");
    };

    registry.set_default_code_source(class_jar_path.display().to_string());
    let binary_name_ref = heap.allocate_string("HelloWorld".to_string());
    let mut sink: Vec<u8> = Vec::new();

    let result = {
        let mut ops = InterpreterCallbackOps {
            registry: &mut registry,
            loader: &loader,
        };
        native_class_for_name_with_loader(
            &[
                Slot::Reference(Some(binary_name_ref)),
                Slot::Int(0),
                Slot::Reference(Some(launched_loader_ref)),
            ],
            &mut heap,
            &mut sink,
            &mut NativeControl::default(),
            &mut ops,
        )
    };

    std::fs::remove_file(&class_jar_path).ok();
    std::fs::remove_file(&empty_jar_path).ok();

    assert!(
        matches!(
            result,
            Err(VmError::JavaException { ref class_name })
                if class_name == "java/lang/ClassNotFoundException"
        ),
        "wrong launched loader should not fall back to registry default code source: {result:?}"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn native_reflect_method_invoke_runs_boot_archive_main() {
    let loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let hello_world = std::fs::read(fixtures_dir().join("HelloWorld.class"))
        .expect("read HelloWorld.class fixture");
    let jar_path = build_test_boot_archive_jar(
        "HelloWorld",
        &[("BOOT-INF/classes/HelloWorld.class", hello_world)],
    );

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.set_default_code_source(jar_path.display().to_string());
    let mut out: Vec<u8> = Vec::new();
    registry
        .ensure_loaded(
            "org/springframework/boot/loader/launch/JarLauncher",
            &loader,
        )
        .expect("load JarLauncher");
    let launcher_ref = heap.allocate(
        "org/springframework/boot/loader/launch/JarLauncher".to_string(),
        total_instance_field_count(
            &registry,
            "org/springframework/boot/loader/launch/JarLauncher",
        ),
    );
    init_object_fields(
        &registry,
        &mut heap,
        launcher_ref,
        "org/springframework/boot/loader/launch/JarLauncher",
    );
    execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/JarLauncher",
        "<init>",
        "()V",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("JarLauncher.<init>() should succeed");
    let urls = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "getClassPathUrls",
        "()Ljava/util/Set;",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("getClassPathUrls should succeed")
    .expect("getClassPathUrls should return a set");
    let Slot::Reference(Some(urls_ref)) = urls else {
        panic!("expected class path set");
    };
    let launched_loader = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "createClassLoader",
        "(Ljava/util/Collection;)Ljava/lang/ClassLoader;",
        &[
            Slot::Reference(Some(launcher_ref)),
            Slot::Reference(Some(urls_ref)),
        ],
    )
    .expect("createClassLoader(Collection) should succeed")
    .expect("createClassLoader(Collection) should return a loader");
    let Slot::Reference(Some(launched_loader_ref)) = launched_loader else {
        panic!("expected launched class loader reference");
    };

    let (class_ref, method_ref) = {
        let binary_name_ref = heap.allocate_string("HelloWorld".to_string());
        let mut ops = InterpreterCallbackOps {
            registry: &mut registry,
            loader: &loader,
        };
        let class_slot = native_class_for_name_with_loader(
            &[
                Slot::Reference(Some(binary_name_ref)),
                Slot::Int(0),
                Slot::Reference(Some(launched_loader_ref)),
            ],
            &mut heap,
            &mut out,
            &mut NativeControl::default(),
            &mut ops,
        )
        .expect("Class.forName should succeed")
        .expect("Class.forName should return a class");
        let Slot::Reference(Some(class_ref)) = class_slot else {
            panic!("expected Class reference");
        };

        let param_array_ref = heap.allocate("[Ljava/lang/Class;".to_string(), 1);
        let string_array_class_ref = allocate_class_object(&mut heap, "[Ljava/lang/String;")
            .expect("allocate array Class");
        heap.get_mut(param_array_ref).unwrap().fields[0] =
            Slot::Reference(Some(string_array_class_ref));
        let method_name_ref = heap.allocate_string("main".to_string());
        let method_slot = native_class_get_declared_method(
            &[
                Slot::Reference(Some(class_ref)),
                Slot::Reference(Some(method_name_ref)),
                Slot::Reference(Some(param_array_ref)),
            ],
            &mut heap,
            &mut out,
            &mut NativeControl::default(),
            &mut ops,
        )
        .expect("getDeclaredMethod should succeed")
        .expect("getDeclaredMethod should return a method");
        let Slot::Reference(Some(method_ref)) = method_slot else {
            panic!("expected Method reference");
        };
        (class_ref, method_ref)
    };

    let _ = class_ref;
    let string_args_ref = heap.allocate("[Ljava/lang/String;".to_string(), 0);
    let invoke_args_ref = heap.allocate("[Ljava/lang/Object;".to_string(), 1);
    heap.get_mut(invoke_args_ref).unwrap().fields[0] = Slot::Reference(Some(string_args_ref));
    let method =
        reflected_method_handle(&heap, method_ref).expect("read reflected method handle");
    let invoke_arg_slots =
        reflection_array_elements(&heap, Slot::Reference(Some(invoke_args_ref)))
            .expect("extract invoke arg slots");
    let invoke_args = build_reflection_invoke_args(
        &heap,
        Slot::Reference(None),
        &method.descriptor,
        invoke_arg_slots,
        method.is_static,
    )
    .expect("build reflection invoke args");
    let result = {
        let mut ops = InterpreterCallbackOps {
            registry: &mut registry,
            loader: &loader,
        };
        ops.ensure_loaded(&method.declaring_class_key)
            .expect("declaring class should stay loadable");
        ops.invoke(
            &mut heap,
            &mut out,
            &method.declaring_class_key,
            &method.method_name,
            &method.descriptor,
            invoke_args,
        )
    };

    std::fs::remove_file(&jar_path).ok();

    assert!(
        result.is_ok(),
        "reflection invoke step should run Boot-archive main, got {result:?}"
    );
    assert_eq!(String::from_utf8(out).unwrap(), "Hello, World!\n");
}

#[test]
fn native_class_get_declared_method_matches_parameter_class_array() {
    struct FixedInspectOps;

    impl CallbackOps for FixedInspectOps {
        fn invoke(
            &mut self,
            _heap: &mut duke_gc::Heap,
            _output: &mut dyn Write,
            _class: &str,
            _method: &str,
            _descriptor: &str,
            _args: Vec<Slot>,
        ) -> VmResult<Option<Slot>> {
            Ok(None)
        }

        fn ensure_loaded(&mut self, _class: &str) -> VmResult<()> {
            Ok(())
        }

        fn inspect_class(&mut self, class: &str) -> VmResult<ReflectedClassInfo> {
            Ok(ReflectedClassInfo {
                internal_name: class.to_string(),
                binary_name: class.replace('/', "."),
                super_class: None,
                interfaces: Vec::new(),
                methods: vec![
                    ReflectedMethodInfo {
                        name: "main".to_string(),
                        descriptor: "([Ljava/lang/String;)V".to_string(),
                        is_public: true,
                        is_static: true,
                    },
                    ReflectedMethodInfo {
                        name: "main".to_string(),
                        descriptor: "()V".to_string(),
                        is_public: true,
                        is_static: true,
                    },
                ],
                fields: Vec::new(),
            })
        }
    }

    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let mut ops = FixedInspectOps;
    let class_ref = allocate_class_object(&mut heap, "com/example/BootApp").unwrap();
    let method_name_ref = heap.allocate_string("main".to_string());
    let param_class_ref = allocate_class_object(&mut heap, "[Ljava/lang/String;").unwrap();
    let params_ref = heap.allocate("[Ljava/lang/Class;".to_string(), 1);
    heap.get_mut(params_ref).unwrap().fields[0] = Slot::Reference(Some(param_class_ref));

    let result = native_class_get_declared_method(
        &[
            Slot::Reference(Some(class_ref)),
            Slot::Reference(Some(method_name_ref)),
            Slot::Reference(Some(params_ref)),
        ],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
        &mut ops,
    )
    .expect("getDeclaredMethod should succeed")
    .expect("getDeclaredMethod should return a Method");

    let Slot::Reference(Some(method_ref)) = result else {
        panic!("expected Method reference");
    };
    let method = reflected_method_handle(&heap, method_ref).unwrap();
    assert_eq!(method.method_name, "main");
    assert_eq!(method.descriptor, "([Ljava/lang/String;)V");
}

#[test]
fn native_reflect_method_get_parameter_count_counts_descriptor_args() {
    let mut heap = duke_gc::Heap::new();
    let method_ref = allocate_reflection_member_object(
        &mut heap,
        "java/lang/reflect/Method",
        "com/example/BootApp",
        "main",
        "([Ljava/lang/String;)V",
        true,
        true,
    )
    .unwrap();
    let mut sink: Vec<u8> = Vec::new();

    let result = native_reflect_method_get_parameter_count(
        &[Slot::Reference(Some(method_ref))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .expect("getParameterCount should succeed");

    assert_eq!(result, Some(Slot::Int(1)));
}

#[test]
fn native_attributes_get_value_reads_manifest_key() {
    let mut heap = duke_gc::Heap::new();
    let raw_ref = heap.allocate_string(
        "Manifest-Version: 1.0\r\nStart-Class: com.example.BootApp\r\n".to_string(),
    );
    let attributes_ref = heap.allocate("java/util/jar/Attributes".to_string(), 1);
    heap.get_mut(attributes_ref).unwrap().fields[0] = Slot::Reference(Some(raw_ref));
    let key_ref = heap.allocate_string("Start-Class".to_string());
    let mut sink: Vec<u8> = Vec::new();

    let result = native_attributes_get_value(
        &[
            Slot::Reference(Some(attributes_ref)),
            Slot::Reference(Some(key_ref)),
        ],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .expect("Attributes.getValue should succeed")
    .expect("Attributes.getValue should return a string");

    let Slot::Reference(Some(value_ref)) = result else {
        panic!("expected String reference");
    };
    assert_eq!(
        string_value_from_ref(&heap, value_ref).unwrap(),
        "com.example.BootApp"
    );
}

#[test]
fn native_thread_current_thread_returns_thread_object() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();

    let result =
        native_thread_current_thread(&[], &mut heap, &mut sink, &mut NativeControl::default())
            .expect("Thread.currentThread should succeed")
            .expect("Thread.currentThread should return a thread");

    let Slot::Reference(Some(thread_ref)) = result else {
        panic!("expected Thread reference");
    };
    let thread = heap.get(thread_ref).unwrap();
    assert_eq!(thread.class_name, "java/lang/Thread");
    assert_eq!(thread.fields[THREAD_ID_SLOT], Slot::Int(-1));
}

#[test]
fn manifest_attribute_value_reads_start_class() {
    let manifest = b"Manifest-Version: 1.0\r\nMain-Class: org.springframework.boot.loader.launch.JarLauncher\r\nStart-Class: com.example.SpringBootSmokeMain\r\n";
    assert_eq!(
        manifest_attribute_value(manifest, "Start-Class"),
        Some("com.example.SpringBootSmokeMain".to_string())
    );
    assert_eq!(
        manifest_attribute_value(manifest, "Main-Class"),
        Some("org.springframework.boot.loader.launch.JarLauncher".to_string())
    );
}

#[test]
fn class_desired_assertion_status_defaults_false() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let desired_assertion_status = match registry.natives_mut().get_kind(
        "java/lang/Class",
        "desiredAssertionStatus",
        "()Z",
    ) {
        Some(HandlerKind::Simple(handler)) => handler,
        other => panic!("expected Class.desiredAssertionStatus native, got {other:?}"),
    };

    let class_ref = allocate_class_object(&mut heap, "java/lang/String").unwrap();
    let mut sink: Vec<u8> = Vec::new();
    let mut control = NativeControl::default();
    let result = desired_assertion_status(
        &[Slot::Reference(Some(class_ref))],
        &mut heap,
        &mut sink,
        &mut control,
    )
    .unwrap();

    assert_eq!(result, Some(Slot::Int(0)));
}

#[test]
fn class_get_package_name_returns_binary_package_name() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let get_package_name = match registry.natives_mut().get_kind(
        "java/lang/Class",
        "getPackageName",
        "()Ljava/lang/String;",
    ) {
        Some(HandlerKind::Simple(handler)) => handler,
        other => panic!("expected Class.getPackageName native, got {other:?}"),
    };

    let class_ref = allocate_class_object(
        &mut heap,
        "org/springframework/boot/loader/net/protocol/Handlers",
    )
    .unwrap();
    let mut sink: Vec<u8> = Vec::new();
    let result = get_package_name(
        &[Slot::Reference(Some(class_ref))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .unwrap();

    let Some(Slot::Reference(Some(package_ref))) = result else {
        panic!("expected package name string");
    };
    assert_eq!(
        string_value_from_ref(&heap, package_ref).unwrap(),
        "org.springframework.boot.loader.net.protocol"
    );
}

#[test]
fn duplicate_binary_name_classes_keep_separate_runtime_loaders() {
    let boot_loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let hello_world = std::fs::read(fixtures_dir().join("HelloWorld.class"))
        .expect("read HelloWorld.class fixture");
    let jar_path_one = build_test_boot_archive_jar(
        "HelloWorld",
        &[("BOOT-INF/classes/HelloWorld.class", hello_world.clone())],
    );
    let jar_path_two = build_test_boot_archive_jar(
        "HelloWorld",
        &[("BOOT-INF/classes/HelloWorld.class", hello_world)],
    );

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let launched_loader_one =
        build_test_launched_loader(&boot_loader, &mut registry, &mut heap, &jar_path_one);
    let launched_loader_two =
        build_test_launched_loader(&boot_loader, &mut registry, &mut heap, &jar_path_two);

    let class_one = load_class_via_launched_loader(
        &boot_loader,
        &mut registry,
        &mut heap,
        "HelloWorld",
        launched_loader_one,
    );
    let class_two = load_class_via_launched_loader(
        &boot_loader,
        &mut registry,
        &mut heap,
        "HelloWorld",
        launched_loader_two,
    );
    let loader_slot_one =
        invoke_class_get_class_loader(&boot_loader, &mut registry, &mut heap, class_one);
    let loader_slot_two =
        invoke_class_get_class_loader(&boot_loader, &mut registry, &mut heap, class_two);

    std::fs::remove_file(&jar_path_one).ok();
    std::fs::remove_file(&jar_path_two).ok();

    assert_eq!(loader_slot_one, Slot::Reference(Some(launched_loader_one)));
    assert_eq!(loader_slot_two, Slot::Reference(Some(launched_loader_two)));
}

#[test]
fn duplicate_binary_name_class_mirrors_are_not_hash_equal() {
    let boot_loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let hello_world = std::fs::read(fixtures_dir().join("HelloWorld.class"))
        .expect("read HelloWorld.class fixture");
    let jar_path_one = build_test_boot_archive_jar(
        "HelloWorld",
        &[("BOOT-INF/classes/HelloWorld.class", hello_world.clone())],
    );
    let jar_path_two = build_test_boot_archive_jar(
        "HelloWorld",
        &[("BOOT-INF/classes/HelloWorld.class", hello_world)],
    );

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let launched_loader_one =
        build_test_launched_loader(&boot_loader, &mut registry, &mut heap, &jar_path_one);
    let launched_loader_two =
        build_test_launched_loader(&boot_loader, &mut registry, &mut heap, &jar_path_two);

    let class_one = load_class_via_launched_loader(
        &boot_loader,
        &mut registry,
        &mut heap,
        "HelloWorld",
        launched_loader_one,
    );
    let class_two = load_class_via_launched_loader(
        &boot_loader,
        &mut registry,
        &mut heap,
        "HelloWorld",
        launched_loader_two,
    );

    std::fs::remove_file(&jar_path_one).ok();
    std::fs::remove_file(&jar_path_two).ok();

    assert!(
        !slots_equal(
            &Slot::Reference(Some(class_one)),
            &Slot::Reference(Some(class_two)),
            &heap,
        ),
        "Class mirrors from different defining loaders must not compare equal"
    );
}

#[test]
fn execute_class_plain_name_rejects_ambiguous_loaded_duplicate_binary_name() {
    let boot_loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let hello_world = std::fs::read(fixtures_dir().join("HelloWorld.class"))
        .expect("read HelloWorld.class fixture");
    let jar_path_one = build_test_boot_archive_jar(
        "HelloWorld",
        &[("BOOT-INF/classes/HelloWorld.class", hello_world.clone())],
    );
    let jar_path_two = build_test_boot_archive_jar(
        "HelloWorld",
        &[("BOOT-INF/classes/HelloWorld.class", hello_world)],
    );

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let launched_loader_one =
        build_test_launched_loader(&boot_loader, &mut registry, &mut heap, &jar_path_one);
    let launched_loader_two =
        build_test_launched_loader(&boot_loader, &mut registry, &mut heap, &jar_path_two);
    let _ = load_class_via_launched_loader(
        &boot_loader,
        &mut registry,
        &mut heap,
        "HelloWorld",
        launched_loader_one,
    );
    let _ = load_class_via_launched_loader(
        &boot_loader,
        &mut registry,
        &mut heap,
        "HelloWorld",
        launched_loader_two,
    );

    let args_ref = heap.allocate("[Ljava/lang/String;".to_string(), 0);
    let err = execute_class(
        &mut registry,
        &boot_loader,
        &mut heap,
        &mut Vec::new(),
        "HelloWorld",
        "main",
        "([Ljava/lang/String;)V",
        &[Slot::Reference(Some(args_ref))],
    )
    .expect_err("plain execute_class should reject ambiguous duplicate definitions");

    std::fs::remove_file(&jar_path_one).ok();
    std::fs::remove_file(&jar_path_two).ok();

    assert!(
        matches!(
            err,
            VmError::AmbiguousClassName { ref name, ref matches }
                if name == "HelloWorld" && matches.len() == 2
        ),
        "expected AmbiguousClassName for duplicate loaded HelloWorld, got {err:?}"
    );
}

#[test]
fn native_class_for_name_rejects_ambiguous_loaded_duplicate_binary_name() {
    let boot_loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let hello_world = std::fs::read(fixtures_dir().join("HelloWorld.class"))
        .expect("read HelloWorld.class fixture");
    let jar_path_one = build_test_boot_archive_jar(
        "HelloWorld",
        &[("BOOT-INF/classes/HelloWorld.class", hello_world.clone())],
    );
    let jar_path_two = build_test_boot_archive_jar(
        "HelloWorld",
        &[("BOOT-INF/classes/HelloWorld.class", hello_world)],
    );

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let launched_loader_one =
        build_test_launched_loader(&boot_loader, &mut registry, &mut heap, &jar_path_one);
    let launched_loader_two =
        build_test_launched_loader(&boot_loader, &mut registry, &mut heap, &jar_path_two);
    let _ = load_class_via_launched_loader(
        &boot_loader,
        &mut registry,
        &mut heap,
        "HelloWorld",
        launched_loader_one,
    );
    let _ = load_class_via_launched_loader(
        &boot_loader,
        &mut registry,
        &mut heap,
        "HelloWorld",
        launched_loader_two,
    );

    let binary_name_ref = heap.allocate_string("HelloWorld".to_string());
    let err = {
        let mut ops = InterpreterCallbackOps {
            registry: &mut registry,
            loader: &boot_loader,
        };
        native_class_for_name(
            &[Slot::Reference(Some(binary_name_ref))],
            &mut heap,
            &mut Vec::new(),
            &mut NativeControl::default(),
            &mut ops,
        )
        .expect_err("Class.forName should reject ambiguous duplicate definitions")
    };

    std::fs::remove_file(&jar_path_one).ok();
    std::fs::remove_file(&jar_path_two).ok();

    assert!(
        matches!(
            err,
            VmError::AmbiguousClassName { ref name, ref matches }
                if name == "HelloWorld" && matches.len() == 2
        ),
        "expected AmbiguousClassName for duplicate Class.forName lookup, got {err:?}"
    );
}

#[test]
fn loader_qualified_instanceof_uses_current_class_provenance() {
    let (
        boot_loader,
        mut registry,
        mut heap,
        jar_path_one,
        jar_path_two,
        class_key_one,
        class_key_two,
    ) = load_duplicate_hello_world_classes();
    install_loader_keyed_hello_world_probe(
        &mut registry,
        &class_key_one,
        "instanceofHelloWorld",
        "(Ljava/lang/Object;)I",
        vec![
            (0, Instruction::Aload0),
            (
                1,
                Instruction::Instanceof(duke_classfile::types::CpIndex(1)),
            ),
            (4, Instruction::Ireturn),
        ],
        vec![],
    );
    let object_ref = allocate_zero_field_object(&registry, &mut heap, &class_key_two);

    let result = execute_class(
        &mut registry,
        &boot_loader,
        &mut heap,
        &mut Vec::new(),
        &class_key_one,
        "instanceofHelloWorld",
        "(Ljava/lang/Object;)I",
        &[Slot::Reference(Some(object_ref))],
    )
    .expect("instanceof probe should execute");

    std::fs::remove_file(&jar_path_one).ok();
    std::fs::remove_file(&jar_path_two).ok();

    assert_eq!(
        result,
        Some(Slot::Int(0)),
        "loader-one instanceof HelloWorld should reject a loader-two HelloWorld object"
    );
}

#[test]
fn loader_qualified_checkcast_uses_current_class_provenance() {
    let (
        boot_loader,
        mut registry,
        mut heap,
        jar_path_one,
        jar_path_two,
        class_key_one,
        class_key_two,
    ) = load_duplicate_hello_world_classes();
    install_loader_keyed_hello_world_probe(
        &mut registry,
        &class_key_one,
        "checkcastHelloWorld",
        "(Ljava/lang/Object;)I",
        vec![
            (0, Instruction::Aload0),
            (1, Instruction::Checkcast(duke_classfile::types::CpIndex(1))),
            (4, Instruction::Pop),
            (5, Instruction::Iconst1),
            (6, Instruction::Ireturn),
        ],
        vec![],
    );
    let object_ref = allocate_zero_field_object(&registry, &mut heap, &class_key_two);

    let err = execute_class(
        &mut registry,
        &boot_loader,
        &mut heap,
        &mut Vec::new(),
        &class_key_one,
        "checkcastHelloWorld",
        "(Ljava/lang/Object;)I",
        &[Slot::Reference(Some(object_ref))],
    )
    .expect_err("checkcast probe should reject loader-two object");

    std::fs::remove_file(&jar_path_one).ok();
    std::fs::remove_file(&jar_path_two).ok();

    assert!(
        matches!(err, VmError::ClassCastException { .. }),
        "expected ClassCastException from loader-qualified checkcast, got {err:?}"
    );
}

#[test]
fn loader_qualified_catch_type_uses_current_class_provenance() {
    let (
        boot_loader,
        mut registry,
        mut heap,
        jar_path_one,
        jar_path_two,
        class_key_one,
        class_key_two,
    ) = load_duplicate_hello_world_classes();
    install_loader_keyed_hello_world_probe(
        &mut registry,
        &class_key_one,
        "catchHelloWorld",
        "(Ljava/lang/Object;)I",
        vec![
            (0, Instruction::Aload0),
            (1, Instruction::Athrow),
            (2, Instruction::Pop),
            (3, Instruction::Iconst1),
            (4, Instruction::Ireturn),
        ],
        vec![ExceptionEntry {
            start_pc: 0,
            end_pc: 2,
            handler_pc: 2,
            catch_type: Some("HelloWorld".to_string()),
        }],
    );
    let object_ref = allocate_zero_field_object(&registry, &mut heap, &class_key_two);

    let err = execute_class(
        &mut registry,
        &boot_loader,
        &mut heap,
        &mut Vec::new(),
        &class_key_one,
        "catchHelloWorld",
        "(Ljava/lang/Object;)I",
        &[Slot::Reference(Some(object_ref))],
    )
    .expect_err("loader-qualified catch should not swallow loader-two throw");

    std::fs::remove_file(&jar_path_one).ok();
    std::fs::remove_file(&jar_path_two).ok();

    assert!(
        matches!(
            err,
            VmError::JavaException { ref class_name } if class_name == &class_key_two
        ),
        "expected uncaught JavaException from loader-two object, got {err:?}"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn native_class_get_class_loader_returns_boot_launched_loader_for_loaded_app_class() {
    let loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let hello_world = std::fs::read(fixtures_dir().join("HelloWorld.class"))
        .expect("read HelloWorld.class fixture");
    let jar_path = build_test_boot_archive_jar(
        "HelloWorld",
        &[("BOOT-INF/classes/HelloWorld.class", hello_world)],
    );

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.set_default_code_source(jar_path.display().to_string());
    registry
        .ensure_loaded(
            "org/springframework/boot/loader/launch/JarLauncher",
            &loader,
        )
        .expect("load JarLauncher");

    let launcher_ref = heap.allocate(
        "org/springframework/boot/loader/launch/JarLauncher".to_string(),
        total_instance_field_count(
            &registry,
            "org/springframework/boot/loader/launch/JarLauncher",
        ),
    );
    init_object_fields(
        &registry,
        &mut heap,
        launcher_ref,
        "org/springframework/boot/loader/launch/JarLauncher",
    );
    execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/JarLauncher",
        "<init>",
        "()V",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("JarLauncher.<init>() should succeed");
    let urls = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "getClassPathUrls",
        "()Ljava/util/Set;",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("getClassPathUrls should succeed")
    .expect("getClassPathUrls should return a set");
    let Slot::Reference(Some(urls_ref)) = urls else {
        panic!("expected class path set");
    };
    let launched_loader = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "createClassLoader",
        "(Ljava/util/Collection;)Ljava/lang/ClassLoader;",
        &[
            Slot::Reference(Some(launcher_ref)),
            Slot::Reference(Some(urls_ref)),
        ],
    )
    .expect("createClassLoader(Collection) should succeed")
    .expect("createClassLoader(Collection) should return a loader");
    let Slot::Reference(Some(launched_loader_ref)) = launched_loader else {
        panic!("expected launched class loader reference");
    };

    let binary_name_ref = heap.allocate_string("HelloWorld".to_string());
    let class_ref = {
        let mut ops = InterpreterCallbackOps {
            registry: &mut registry,
            loader: &loader,
        };
        let class_slot = native_class_for_name_with_loader(
            &[
                Slot::Reference(Some(binary_name_ref)),
                Slot::Int(0),
                Slot::Reference(Some(launched_loader_ref)),
            ],
            &mut heap,
            &mut Vec::new(),
            &mut NativeControl::default(),
            &mut ops,
        )
        .expect("Class.forName should succeed")
        .expect("Class.forName should return a class");
        let Slot::Reference(Some(class_ref)) = class_slot else {
            panic!("expected Class reference");
        };
        class_ref
    };

    let get_class_loader_result = match registry.natives_mut().get_kind(
        "java/lang/Class",
        "getClassLoader",
        "()Ljava/lang/ClassLoader;",
    ) {
        Some(HandlerKind::Simple(handler)) => handler(
            &[Slot::Reference(Some(class_ref))],
            &mut heap,
            &mut Vec::new(),
            &mut NativeControl::default(),
        ),
        Some(HandlerKind::Callback(handler)) => {
            let mut ops = InterpreterCallbackOps {
                registry: &mut registry,
                loader: &loader,
            };
            handler(
                &[Slot::Reference(Some(class_ref))],
                &mut heap,
                &mut Vec::new(),
                &mut NativeControl::default(),
                &mut ops,
            )
        }
        other => panic!("expected Class.getClassLoader native, got {other:?}"),
    }
    .expect("Class.getClassLoader should succeed")
    .expect("Class.getClassLoader should return a ClassLoader");

    std::fs::remove_file(&jar_path).ok();

    assert_eq!(
        get_class_loader_result,
        Slot::Reference(Some(launched_loader_ref)),
        "Class.getClassLoader should return the launched loader for app classes"
    );
}

#[test]
fn url_set_url_stream_handler_factory_is_noop() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let set_factory = match registry.natives_mut().get_kind(
        "java/net/URL",
        "setURLStreamHandlerFactory",
        "(Ljava/net/URLStreamHandlerFactory;)V",
    ) {
        Some(HandlerKind::Simple(handler)) => handler,
        other => panic!("expected URL.setURLStreamHandlerFactory native, got {other:?}"),
    };
    let mut sink: Vec<u8> = Vec::new();
    let result = set_factory(
        &[Slot::Reference(None)],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .unwrap();

    assert_eq!(result, None);
}

#[test]
fn cds_bootstrap_flags_default_false() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let mut sink: Vec<u8> = Vec::new();
    let mut control = NativeControl::default();
    for method in [
        "isDumpingClassList0",
        "isDumpingArchive0",
        "isSharingEnabled0",
    ] {
        let handler =
            match registry
                .natives_mut()
                .get_kind("jdk/internal/misc/CDS", method, "()Z")
            {
                Some(HandlerKind::Simple(handler)) => handler,
                other => panic!("expected CDS native {method}, got {other:?}"),
            };
        let result = handler(&[], &mut heap, &mut sink, &mut control).unwrap();
        assert_eq!(result, Some(Slot::Int(0)), "{method} should default false");
    }
}

#[test]
fn cds_random_seed_for_dumping_defaults_zero() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let handler = match registry.natives_mut().get_kind(
        "jdk/internal/misc/CDS",
        "getRandomSeedForDumping",
        "()J",
    ) {
        Some(HandlerKind::Simple(handler)) => handler,
        other => panic!("expected CDS.getRandomSeedForDumping native, got {other:?}"),
    };
    let mut sink: Vec<u8> = Vec::new();
    let mut control = NativeControl::default();
    let result = handler(&[], &mut heap, &mut sink, &mut control).unwrap();
    assert_eq!(result, Some(Slot::Long(0)));
}

#[test]
fn cds_archive_hooks_are_noops() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let class_ref = allocate_class_object(&mut heap, "java/lang/String").unwrap();
    let string_ref = heap.allocate_string("lambda".to_string());
    let mut sink: Vec<u8> = Vec::new();
    let mut control = NativeControl::default();
    for (method, descriptor, args) in [
        (
            "initializeFromArchive",
            "(Ljava/lang/Class;)V",
            vec![Slot::Reference(Some(class_ref))],
        ),
        (
            "defineArchivedModules",
            "(Ljava/lang/ClassLoader;Ljava/lang/ClassLoader;)V",
            vec![Slot::Reference(None), Slot::Reference(None)],
        ),
        (
            "logLambdaFormInvoker",
            "(Ljava/lang/String;)V",
            vec![Slot::Reference(Some(string_ref))],
        ),
    ] {
        let handler =
            match registry
                .natives_mut()
                .get_kind("jdk/internal/misc/CDS", method, descriptor)
            {
                Some(HandlerKind::Simple(handler)) => handler,
                other => panic!("expected CDS native {method}, got {other:?}"),
            };
        let result = handler(&args, &mut heap, &mut sink, &mut control).unwrap();
        assert_eq!(result, None, "{method} should be a no-op");
    }
}

#[test]
fn multi_class_jar_loading() {
    let jar_path = fixtures_dir().join("multi.jar");
    let loader = duke_loader::ZipLoader::open(&jar_path).expect("open multi.jar");
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    // Pre-load the entry class from the JAR.
    registry
        .ensure_loaded("MultiClassJar", &loader)
        .expect("load MultiClassJar");
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "MultiClassJar",
        "compute",
        "()I",
        &[],
    )
    .expect("MultiClassJar.compute should succeed");
    assert_eq!(result, Some(Slot::Int(99)));
}

#[test]
fn boot_layout_jar_loads_dependency_from_nested_lib() {
    let fixtures = fixtures_dir();
    let outer_class = std::fs::read(fixtures.join("MultiClassJar.class"))
        .expect("read MultiClassJar.class fixture");
    let nested_class =
        std::fs::read(fixtures.join("JarHelper.class")).expect("read JarHelper.class fixture");
    let nested_jar = build_test_multi_entry_zip(&[("JarHelper.class", &nested_class)]);
    let outer_jar = build_test_multi_entry_zip(&[
        ("BOOT-INF/classes/MultiClassJar.class", &outer_class),
        ("BOOT-INF/lib/helper.jar", &nested_jar),
    ]);

    let jar_path = std::env::temp_dir().join("duke_test_boot_layout_nested_lib.jar");
    std::fs::write(&jar_path, &outer_jar).expect("write boot layout test jar");

    let loader = duke_loader::ZipLoader::open(&jar_path).expect("open boot layout jar");
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry
        .ensure_loaded("MultiClassJar", &loader)
        .expect("load MultiClassJar from BOOT-INF/classes");
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "MultiClassJar",
        "compute",
        "()I",
        &[],
    )
    .expect("MultiClassJar.compute should resolve JarHelper from nested BOOT-INF/lib jar");

    std::fs::remove_file(&jar_path).ok();
    assert_eq!(result, Some(Slot::Int(99)));
}

#[test]
fn execute_class_uses_loaded_class_code_source_for_nested_boot_lib_dependency() {
    let boot_loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let fixtures = fixtures_dir();
    let outer_class = std::fs::read(fixtures.join("MultiClassJar.class"))
        .expect("read MultiClassJar.class fixture");
    let nested_class =
        std::fs::read(fixtures.join("JarHelper.class")).expect("read JarHelper.class fixture");
    let nested_jar = build_test_multi_entry_zip(&[("JarHelper.class", &nested_class)]);
    let jar_path = build_test_boot_archive_jar(
        "MultiClassJar",
        &[
            ("BOOT-INF/classes/MultiClassJar.class", outer_class),
            ("BOOT-INF/lib/helper.jar", nested_jar),
        ],
    );

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    assert!(
        registry
            .ensure_loaded_with_code_source("MultiClassJar", &jar_path.display().to_string())
            .expect("load MultiClassJar via archive provenance"),
        "boot archive code source should load the entry class"
    );

    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &boot_loader,
        &mut heap,
        &mut out,
        "MultiClassJar",
        "compute",
        "()I",
        &[],
    )
    .expect("MultiClassJar.compute should inherit its boot archive code source");

    std::fs::remove_file(&jar_path).ok();
    assert_eq!(result, Some(Slot::Int(99)));
}

#[test]
#[allow(clippy::too_many_lines)]
fn boot_nested_dependency_inherits_launched_runtime_loader() {
    let boot_loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let fixtures = fixtures_dir();
    let outer_class = std::fs::read(fixtures.join("MultiClassJar.class"))
        .expect("read MultiClassJar.class fixture");
    let nested_class =
        std::fs::read(fixtures.join("JarHelper.class")).expect("read JarHelper.class fixture");
    let nested_jar = build_test_multi_entry_zip(&[("JarHelper.class", &nested_class)]);
    let jar_path = build_test_boot_archive_jar(
        "MultiClassJar",
        &[
            ("BOOT-INF/classes/MultiClassJar.class", outer_class),
            ("BOOT-INF/lib/helper.jar", nested_jar),
        ],
    );

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.set_default_code_source(jar_path.display().to_string());
    registry
        .ensure_loaded(
            "org/springframework/boot/loader/launch/JarLauncher",
            &boot_loader,
        )
        .expect("load JarLauncher");

    let launcher_ref = heap.allocate(
        "org/springframework/boot/loader/launch/JarLauncher".to_string(),
        total_instance_field_count(
            &registry,
            "org/springframework/boot/loader/launch/JarLauncher",
        ),
    );
    init_object_fields(
        &registry,
        &mut heap,
        launcher_ref,
        "org/springframework/boot/loader/launch/JarLauncher",
    );
    execute_class(
        &mut registry,
        &boot_loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/JarLauncher",
        "<init>",
        "()V",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("JarLauncher.<init>() should succeed");
    let urls = execute_class(
        &mut registry,
        &boot_loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "getClassPathUrls",
        "()Ljava/util/Set;",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("getClassPathUrls should succeed")
    .expect("getClassPathUrls should return a set");
    let Slot::Reference(Some(urls_ref)) = urls else {
        panic!("expected class path set");
    };
    let launched_loader = execute_class(
        &mut registry,
        &boot_loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "createClassLoader",
        "(Ljava/util/Collection;)Ljava/lang/ClassLoader;",
        &[
            Slot::Reference(Some(launcher_ref)),
            Slot::Reference(Some(urls_ref)),
        ],
    )
    .expect("createClassLoader(Collection) should succeed")
    .expect("createClassLoader(Collection) should return a loader");
    let Slot::Reference(Some(launched_loader_ref)) = launched_loader else {
        panic!("expected launched class loader reference");
    };

    let binary_name_ref = heap.allocate_string("MultiClassJar".to_string());
    let entry_class_ref = {
        let mut ops = InterpreterCallbackOps {
            registry: &mut registry,
            loader: &boot_loader,
        };
        let class_slot = native_class_for_name_with_loader(
            &[
                Slot::Reference(Some(binary_name_ref)),
                Slot::Int(0),
                Slot::Reference(Some(launched_loader_ref)),
            ],
            &mut heap,
            &mut Vec::new(),
            &mut NativeControl::default(),
            &mut ops,
        )
        .expect("Class.forName should succeed")
        .expect("Class.forName should return a class");
        let Slot::Reference(Some(class_ref)) = class_slot else {
            panic!("expected Class reference");
        };
        class_ref
    };
    let entry_class_key = class_key_from_ref(&heap, entry_class_ref)
        .expect("class mirror should carry a class key");

    assert_eq!(
        registry.runtime_loader_for_class(&entry_class_key),
        Some(launched_loader_ref),
        "entry class should remember the launched runtime loader"
    );

    let result = execute_class(
        &mut registry,
        &boot_loader,
        &mut heap,
        &mut Vec::new(),
        &entry_class_key,
        "compute",
        "()I",
        &[],
    )
    .expect("MultiClassJar.compute should succeed with launched loader provenance");

    std::fs::remove_file(&jar_path).ok();

    assert_eq!(result, Some(Slot::Int(99)));
    let jar_path_string = jar_path.display().to_string();
    let helper_class_key = registry.class_key_from_provenance(
        "JarHelper",
        Some(&jar_path_string),
        Some(launched_loader_ref),
    );
    assert_eq!(
        registry.runtime_loader_for_class(&helper_class_key),
        Some(launched_loader_ref),
        "nested boot dependency should inherit the launched runtime loader"
    );
}

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn boot_archive_create_file_returns_jar_file_archive_object() {
    let loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry
        .ensure_loaded("org/springframework/boot/loader/launch/Archive", &loader)
        .expect("load Archive");

    let file_ref = heap.allocate("java/io/File".to_string(), 1);
    let path_ref = heap.allocate_string(fixtures_dir().join("hello.jar").display().to_string());
    heap.get_mut(file_ref).unwrap().fields[0] = Slot::Reference(Some(path_ref));

    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "org/springframework/boot/loader/launch/Archive",
        "create",
        "(Ljava/io/File;)Lorg/springframework/boot/loader/launch/Archive;",
        &[Slot::Reference(Some(file_ref))],
    )
    .expect("Archive.create(File) should succeed")
    .expect("Archive.create(File) should return an archive");

    let Slot::Reference(Some(archive_ref)) = result else {
        panic!("expected Archive reference");
    };
    assert_eq!(
        heap.get(archive_ref).unwrap().class_name,
        "org/springframework/boot/loader/launch/JarFileArchive"
    );
}

#[test]
fn boot_jar_launcher_default_constructor_initializes_archive_field() {
    let loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.set_default_code_source(fixtures_dir().join("hello.jar").display().to_string());
    registry
        .ensure_loaded(
            "org/springframework/boot/loader/launch/JarLauncher",
            &loader,
        )
        .expect("load JarLauncher");

    let launcher_ref = heap.allocate(
        "org/springframework/boot/loader/launch/JarLauncher".to_string(),
        total_instance_field_count(
            &registry,
            "org/springframework/boot/loader/launch/JarLauncher",
        ),
    );
    init_object_fields(
        &registry,
        &mut heap,
        launcher_ref,
        "org/springframework/boot/loader/launch/JarLauncher",
    );

    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "org/springframework/boot/loader/launch/JarLauncher",
        "<init>",
        "()V",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("JarLauncher.<init>() should succeed");
    assert_eq!(result, None);

    let archive_slot_idx = field_slot_idx(
        &registry,
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "archive",
    )
    .expect("archive field slot");
    let Slot::Reference(Some(archive_ref)) =
        heap.get(launcher_ref).unwrap().fields[archive_slot_idx]
    else {
        panic!("expected archive reference field");
    };
    assert_eq!(
        heap.get(archive_ref).unwrap().class_name,
        "org/springframework/boot/loader/launch/JarFileArchive"
    );
}

#[test]
fn boot_jar_launcher_get_main_class_reads_start_class_manifest_entry() {
    let loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let jar_bytes = build_test_multi_entry_zip(&[(
        "META-INF/MANIFEST.MF",
        b"Manifest-Version: 1.0\r\nStart-Class: com.example.TestApp\r\n",
    )]);
    let jar_path = std::env::temp_dir().join(format!(
        "duke_test_boot_start_class_{}.jar",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    std::fs::write(&jar_path, jar_bytes).expect("write boot start-class jar");

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.set_default_code_source(jar_path.display().to_string());
    registry
        .ensure_loaded(
            "org/springframework/boot/loader/launch/JarLauncher",
            &loader,
        )
        .expect("load JarLauncher");

    let launcher_ref = heap.allocate(
        "org/springframework/boot/loader/launch/JarLauncher".to_string(),
        total_instance_field_count(
            &registry,
            "org/springframework/boot/loader/launch/JarLauncher",
        ),
    );
    init_object_fields(
        &registry,
        &mut heap,
        launcher_ref,
        "org/springframework/boot/loader/launch/JarLauncher",
    );
    execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/JarLauncher",
        "<init>",
        "()V",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("JarLauncher.<init>() should succeed");

    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "getMainClass",
        "()Ljava/lang/String;",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("JarLauncher.getMainClass should succeed")
    .expect("JarLauncher.getMainClass should return a string");

    std::fs::remove_file(&jar_path).ok();

    let Slot::Reference(Some(string_ref)) = result else {
        panic!("expected String reference");
    };
    assert_eq!(
        string_value_from_ref(&heap, string_ref).unwrap(),
        "com.example.TestApp"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn boot_launcher_pre_main_sequence_preserves_archive_field() {
    let loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let jar_bytes = build_test_multi_entry_zip(&[
        (
            "META-INF/MANIFEST.MF",
            b"Manifest-Version: 1.0\r\nStart-Class: com.example.TestApp\r\n",
        ),
        ("BOOT-INF/classes/", b""),
    ]);
    let jar_path = std::env::temp_dir().join(format!(
        "duke_test_boot_pre_main_{}.jar",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    std::fs::write(&jar_path, jar_bytes).expect("write boot pre-main jar");

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.set_default_code_source(jar_path.display().to_string());
    registry
        .ensure_loaded(
            "org/springframework/boot/loader/launch/JarLauncher",
            &loader,
        )
        .expect("load JarLauncher");

    let launcher_ref = heap.allocate(
        "org/springframework/boot/loader/launch/JarLauncher".to_string(),
        total_instance_field_count(
            &registry,
            "org/springframework/boot/loader/launch/JarLauncher",
        ),
    );
    init_object_fields(
        &registry,
        &mut heap,
        launcher_ref,
        "org/springframework/boot/loader/launch/JarLauncher",
    );
    execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/JarLauncher",
        "<init>",
        "()V",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("JarLauncher.<init>() should succeed");

    let urls = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "getClassPathUrls",
        "()Ljava/util/Set;",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("getClassPathUrls should succeed")
    .expect("getClassPathUrls should return a set");
    let Slot::Reference(Some(urls_ref)) = urls else {
        panic!("expected class path set");
    };

    let _class_loader = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "createClassLoader",
        "(Ljava/util/Collection;)Ljava/lang/ClassLoader;",
        &[
            Slot::Reference(Some(launcher_ref)),
            Slot::Reference(Some(urls_ref)),
        ],
    )
    .expect("createClassLoader(Collection) should succeed")
    .expect("createClassLoader(Collection) should return a loader");

    let archive_slot_idx = field_slot_idx(
        &registry,
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "archive",
    )
    .expect("archive field slot");
    let Slot::Reference(Some(archive_ref)) =
        heap.get(launcher_ref).unwrap().fields[archive_slot_idx]
    else {
        panic!("expected archive reference after pre-main sequence");
    };
    assert_eq!(
        heap.get(archive_ref).unwrap().class_name,
        "org/springframework/boot/loader/launch/JarFileArchive"
    );

    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "getMainClass",
        "()Ljava/lang/String;",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("getMainClass should succeed")
    .expect("getMainClass should return a string");

    std::fs::remove_file(&jar_path).ok();

    let Slot::Reference(Some(string_ref)) = result else {
        panic!("expected String reference");
    };
    assert_eq!(
        string_value_from_ref(&heap, string_ref).unwrap(),
        "com.example.TestApp"
    );
}

#[test]
fn boot_executable_jar_main_runs_from_boot_inf_classes() {
    let loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let hello_world = std::fs::read(fixtures_dir().join("HelloWorld.class"))
        .expect("read HelloWorld.class fixture");
    let filler_nested_jar = build_test_multi_entry_zip(&[]);
    let mut archive_entries =
        vec![("BOOT-INF/classes/HelloWorld.class".to_string(), hello_world)];
    for i in 0..384 {
        archive_entries.push((
            format!("BOOT-INF/lib/filler-{i:03}.jar"),
            filler_nested_jar.clone(),
        ));
    }
    let archive_entry_refs: Vec<(&str, Vec<u8>)> = archive_entries
        .iter()
        .map(|(name, bytes)| (name.as_str(), bytes.clone()))
        .collect();
    let jar_path = build_test_boot_archive_jar("HelloWorld", &archive_entry_refs);

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.set_default_code_source(jar_path.display().to_string());
    registry
        .ensure_loaded(
            "org/springframework/boot/loader/launch/JarLauncher",
            &loader,
        )
        .expect("load JarLauncher");

    let args_ref = heap.allocate("[Ljava/lang/String;".to_string(), 0);
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut out,
        "org/springframework/boot/loader/launch/JarLauncher",
        "main",
        "([Ljava/lang/String;)V",
        &[Slot::Reference(Some(args_ref))],
    );

    std::fs::remove_file(&jar_path).ok();

    assert!(
        result.is_ok(),
        "boot executable jar main should run successfully, got {result:?}"
    );
    assert_eq!(
        String::from_utf8(out).expect("utf8 output"),
        "Hello, World!\n"
    );
}

#[test]
fn boot_class_path_urls_collection_to_array_returns_url_array() {
    let loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let hello_world = std::fs::read(fixtures_dir().join("HelloWorld.class"))
        .expect("read HelloWorld.class fixture");
    let filler_nested_jar = build_test_multi_entry_zip(&[]);
    let mut archive_entries =
        vec![("BOOT-INF/classes/HelloWorld.class".to_string(), hello_world)];
    for i in 0..384 {
        archive_entries.push((
            format!("BOOT-INF/lib/filler-{i:03}.jar"),
            filler_nested_jar.clone(),
        ));
    }
    let archive_entry_refs: Vec<(&str, Vec<u8>)> = archive_entries
        .iter()
        .map(|(name, bytes)| (name.as_str(), bytes.clone()))
        .collect();
    let jar_path = build_test_boot_archive_jar("HelloWorld", &archive_entry_refs);

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.set_default_code_source(jar_path.display().to_string());
    registry
        .ensure_loaded(
            "org/springframework/boot/loader/launch/JarLauncher",
            &loader,
        )
        .expect("load JarLauncher");

    let launcher_ref = heap.allocate(
        "org/springframework/boot/loader/launch/JarLauncher".to_string(),
        total_instance_field_count(
            &registry,
            "org/springframework/boot/loader/launch/JarLauncher",
        ),
    );
    init_object_fields(
        &registry,
        &mut heap,
        launcher_ref,
        "org/springframework/boot/loader/launch/JarLauncher",
    );
    execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/JarLauncher",
        "<init>",
        "()V",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("JarLauncher.<init>() should succeed");

    let urls = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "getClassPathUrls",
        "()Ljava/util/Set;",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("getClassPathUrls should succeed")
    .expect("getClassPathUrls should return a set");
    let Slot::Reference(Some(urls_ref)) = urls else {
        panic!("expected HashSet reference");
    };
    assert_eq!(heap.get(urls_ref).unwrap().class_name, "java/util/HashSet");

    let seed_ref = heap.allocate("[Ljava/net/URL;".to_string(), 0);
    let array = native_collection_to_array_with_seed_array(
        &[
            Slot::Reference(Some(urls_ref)),
            Slot::Reference(Some(seed_ref)),
        ],
        &mut heap,
        &mut Vec::new(),
        &mut NativeControl::default(),
    )
    .expect("Collection.toArray(Object[]) should succeed")
    .expect("Collection.toArray(Object[]) should return an array");

    std::fs::remove_file(&jar_path).ok();

    let Slot::Reference(Some(array_ref)) = array else {
        panic!("expected URL[] reference");
    };
    assert_eq!(heap.get(array_ref).unwrap().class_name, "[Ljava/net/URL;");
}

#[test]
#[allow(clippy::too_many_lines)]
fn boot_create_class_loader_preserves_archive_field_on_large_archive() {
    let loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let hello_world = std::fs::read(fixtures_dir().join("HelloWorld.class"))
        .expect("read HelloWorld.class fixture");
    let filler_nested_jar = build_test_multi_entry_zip(&[]);
    let mut archive_entries =
        vec![("BOOT-INF/classes/HelloWorld.class".to_string(), hello_world)];
    for i in 0..384 {
        archive_entries.push((
            format!("BOOT-INF/lib/filler-{i:03}.jar"),
            filler_nested_jar.clone(),
        ));
    }
    let archive_entry_refs: Vec<(&str, Vec<u8>)> = archive_entries
        .iter()
        .map(|(name, bytes)| (name.as_str(), bytes.clone()))
        .collect();
    let jar_path = build_test_boot_archive_jar("HelloWorld", &archive_entry_refs);

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.set_default_code_source(jar_path.display().to_string());
    registry
        .ensure_loaded(
            "org/springframework/boot/loader/launch/JarLauncher",
            &loader,
        )
        .expect("load JarLauncher");

    let mut launcher_ref = heap.allocate(
        "org/springframework/boot/loader/launch/JarLauncher".to_string(),
        total_instance_field_count(
            &registry,
            "org/springframework/boot/loader/launch/JarLauncher",
        ),
    );
    init_object_fields(
        &registry,
        &mut heap,
        launcher_ref,
        "org/springframework/boot/loader/launch/JarLauncher",
    );
    execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/JarLauncher",
        "<init>",
        "()V",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("JarLauncher.<init>() should succeed");

    let urls = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "getClassPathUrls",
        "()Ljava/util/Set;",
        &[Slot::Reference(Some(launcher_ref))],
    )
    .expect("getClassPathUrls should succeed")
    .expect("getClassPathUrls should return a set");
    let Slot::Reference(Some(mut urls_ref)) = urls else {
        panic!("expected class path set");
    };

    let _class_loader = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut Vec::new(),
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "createClassLoader",
        "(Ljava/util/Collection;)Ljava/lang/ClassLoader;",
        &[
            Slot::Reference(Some(launcher_ref)),
            Slot::Reference(Some(urls_ref)),
        ],
    )
    .expect("createClassLoader(Collection) should succeed")
    .expect("createClassLoader(Collection) should return a loader");

    patch_forwarded_ref_if_needed(&heap, &mut launcher_ref);
    patch_forwarded_ref_if_needed(&heap, &mut urls_ref);
    std::fs::remove_file(&jar_path).ok();

    let archive_slot_idx = field_slot_idx(
        &registry,
        "org/springframework/boot/loader/launch/ExecutableArchiveLauncher",
        "archive",
    )
    .expect("archive field slot");
    let Slot::Reference(Some(archive_ref)) =
        heap.get(launcher_ref).unwrap().fields[archive_slot_idx]
    else {
        panic!("expected archive reference after class loader creation");
    };
    assert_eq!(
        heap.get(archive_ref).unwrap().class_name,
        "org/springframework/boot/loader/launch/JarFileArchive"
    );
}

#[test]
fn zip_file_native_entry_count() {
    let jar_path = fixtures_dir().join("hello.jar");
    let jar_str = jar_path.to_str().unwrap().to_string();
    let result = run_bootstrap_with_string_args(
        "ZipReadTest.class",
        "entryCount",
        "(Ljava/lang/String;)I",
        &[jar_str],
    )
    .expect("entryCount should succeed");
    // hello.jar contains HelloWorld.class + META-INF/MANIFEST.MF
    if let Some(Slot::Int(count)) = result {
        assert!(count >= 2, "expected at least 2 entries, got {count}");
    } else {
        panic!("expected Int result, got {result:?}");
    }
}

#[test]
fn zip_file_native_read_first_byte() {
    // Use hello.jar — read the first byte of HelloWorld.class (0xCA = 202).
    let jar_path = fixtures_dir().join("hello.jar");
    let path_str = jar_path.to_str().unwrap().to_string();
    let result = run_bootstrap_with_string_args(
        "ZipReadTest.class",
        "readFirstByte",
        "(Ljava/lang/String;Ljava/lang/String;)I",
        &[path_str, "HelloWorld.class".to_string()],
    )
    .expect("readFirstByte should succeed");
    // 0xCA = 202 (first byte of .class magic number)
    assert_eq!(result, Some(Slot::Int(0xCA)));
}

#[test]
fn jar_file_native_init_accepts_file_objects() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let init = match registry.natives_mut().get_kind(
        "java/util/jar/JarFile",
        "<init>",
        "(Ljava/io/File;)V",
    ) {
        Some(HandlerKind::Simple(handler)) => handler,
        other => panic!("expected JarFile.<init>(File) native, got {other:?}"),
    };
    let close = match registry
        .natives_mut()
        .get_kind("java/util/zip/ZipFile", "close", "()V")
    {
        Some(HandlerKind::Simple(handler)) => handler,
        other => panic!("expected ZipFile.close native, got {other:?}"),
    };

    let jar_ref = heap.allocate("java/util/jar/JarFile".to_string(), 1);
    let file_ref = heap.allocate("java/io/File".to_string(), 1);
    let path_ref = heap.allocate_string(fixtures_dir().join("hello.jar").display().to_string());
    heap.get_mut(file_ref).unwrap().fields[0] = Slot::Reference(Some(path_ref));

    let mut sink: Vec<u8> = Vec::new();
    let mut control = NativeControl::default();
    let result = init(
        &[
            Slot::Reference(Some(jar_ref)),
            Slot::Reference(Some(file_ref)),
        ],
        &mut heap,
        &mut sink,
        &mut control,
    )
    .expect("JarFile.<init>(File) should open hello.jar");

    assert_eq!(result, None);
    assert!(
        extract_io_fd(&heap, jar_ref).is_ok(),
        "JarFile fd should be initialized"
    );
    close(
        &[Slot::Reference(Some(jar_ref))],
        &mut heap,
        &mut sink,
        &mut control,
    )
    .expect("ZipFile.close should succeed");
}
