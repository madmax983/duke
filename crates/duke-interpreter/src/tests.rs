use super::*;
use sha2::Digest as _;

macro_rules! wrap_simple_native_for_tests {
    ($($name:ident),* $(,)?) => {
        $(
            fn $name(
                args: &[Slot],
                heap: &mut duke_gc::Heap,
                out: &mut dyn std::io::Write,
            ) -> Result<Option<Slot>> {
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
    native_string_equalsignorecase,
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
) -> Result<Option<Slot>> {
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
    ) -> Result<Option<Slot>> {
        Ok(None)
    }

    fn ensure_loaded(&mut self, _class: &str) -> Result<()> {
        Ok(())
    }

    fn inspect_class(&mut self, _class: &str) -> Result<ReflectedClassInfo> {
        Ok(ReflectedClassInfo {
            internal_name: String::new(),
            binary_name: String::new(),
            super_class: None,
            interfaces: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            access_flags: 0,
            annotations: Vec::new(),
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
    ) -> Result<Option<Slot>> {
        Ok(None)
    }

    fn ensure_loaded(&mut self, _class: &str) -> Result<()> {
        Ok(())
    }

    fn inspect_class(&mut self, _class: &str) -> Result<ReflectedClassInfo> {
        Ok(ReflectedClassInfo {
            internal_name: String::new(),
            binary_name: String::new(),
            super_class: None,
            interfaces: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            access_flags: 0,
            annotations: Vec::new(),
        })
    }

    fn code_source_for_class(&mut self, _class: &str) -> Result<Option<String>> {
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
        ) -> Result<Option<Slot>> {
            assert_eq!(class, self.collection_class);
            assert_eq!(method, "toArray");
            assert_eq!(descriptor, "()[Ljava/lang/Object;");
            Ok(Some(Slot::Reference(Some(self.array_ref))))
        }

        fn ensure_loaded(&mut self, _class: &str) -> Result<()> {
            Ok(())
        }

        fn inspect_class(&mut self, _class: &str) -> Result<ReflectedClassInfo> {
            Ok(ReflectedClassInfo {
                internal_name: String::new(),
                binary_name: String::new(),
                super_class: None,
                interfaces: Vec::new(),
                methods: Vec::new(),
                fields: Vec::new(),
                access_flags: 0,
                annotations: Vec::new(),
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
fn message_digest_sha256_get_instance_and_digest_bytes() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let mut out = Vec::new();

    let algo_ref = heap.allocate_string("SHA-256".to_string());
    let get_instance = registry
        .natives()
        .get(
            "java/security/MessageDigest",
            "getInstance",
            "(Ljava/lang/String;)Ljava/security/MessageDigest;",
        )
        .expect("MessageDigest.getInstance native should be registered");
    let digest_ref = match get_instance(
        &[Slot::Reference(Some(algo_ref))],
        &mut heap,
        &mut out,
        &mut NativeControl::default(),
    )
    .expect("getInstance should succeed")
    {
        Some(Slot::Reference(Some(r))) => r,
        other => panic!("unexpected getInstance return: {other:?}"),
    };

    let input_ref = heap.allocate("[B".to_string(), 3);
    heap.get_mut(input_ref).unwrap().fields = vec![Slot::Int(97), Slot::Int(98), Slot::Int(99)];
    let digest_bytes = registry
        .natives()
        .get("java/security/MessageDigest", "digest", "([B)[B")
        .expect("MessageDigest.digest([B)[B native should be registered");
    let result_ref = match digest_bytes(
        &[
            Slot::Reference(Some(digest_ref)),
            Slot::Reference(Some(input_ref)),
        ],
        &mut heap,
        &mut out,
        &mut NativeControl::default(),
    )
    .expect("digest should succeed")
    {
        Some(Slot::Reference(Some(r))) => r,
        other => panic!("unexpected digest return: {other:?}"),
    };

    let result: Vec<u8> = heap
        .get(result_ref)
        .unwrap()
        .fields
        .iter()
        .map(|slot| match slot {
            Slot::Int(v) => v.to_le_bytes()[0],
            other => panic!("unexpected digest byte slot: {other:?}"),
        })
        .collect();
    let expected = sha2::Sha256::digest(b"abc").to_vec();
    assert_eq!(result, expected);
}

#[test]
fn secure_random_next_bytes_populates_array() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let mut out = Vec::new();

    let secure_random_ref = heap.allocate("java/security/SecureRandom".to_string(), 0);
    let bytes_ref = heap.allocate("[B".to_string(), 32);
    let next_bytes = registry
        .natives()
        .get("java/security/SecureRandom", "nextBytes", "([B)V")
        .expect("SecureRandom.nextBytes native should be registered");
    next_bytes(
        &[
            Slot::Reference(Some(secure_random_ref)),
            Slot::Reference(Some(bytes_ref)),
        ],
        &mut heap,
        &mut out,
        &mut NativeControl::default(),
    )
    .expect("nextBytes should succeed");

    let generated = &heap.get(bytes_ref).unwrap().fields;
    assert_eq!(generated.len(), 32);
    assert!(generated.iter().all(|slot| matches!(slot, Slot::Int(_))));
}

#[test]
fn crypto_spec_sha256_digest_runs_through_java_get_instance() {
    assert_eq!(
        run_bootstrap_int("CryptoSpecTest.class", "testSha256Digest", "()I"),
        1
    );
}

#[test]
fn crypto_spec_md5_lookup_is_case_insensitive() {
    assert_eq!(
        run_bootstrap_int(
            "CryptoSpecTest.class",
            "testMd5DigestCaseInsensitiveLookup",
            "()I"
        ),
        1
    );
}

#[test]
fn crypto_spec_update_then_digest_buffers_bytes() {
    assert_eq!(
        run_bootstrap_int("CryptoSpecTest.class", "testUpdateThenDigest", "()I"),
        1
    );
}

#[test]
fn crypto_spec_secure_random_instantiates_and_generates_bytes() {
    assert_eq!(
        run_bootstrap_int(
            "CryptoSpecTest.class",
            "testSecureRandomInstantiation",
            "()I"
        ),
        1
    );
}

#[test]
fn crypto_spec_provider_registration_is_visible_to_standard_apis() {
    assert_eq!(
        run_bootstrap_int("CryptoSpecTest.class", "testProviderRegistration", "()I"),
        1
    );
}

#[test]
fn access_controller_privileged_action_returns_string() {
    assert_eq!(
        run_bootstrap_int_completion(
            "AccessControllerDoPrivilegedTest.class",
            "privilegedActionReturnsString",
            "()I"
        ),
        1
    );
}

#[test]
fn access_controller_privileged_action_returns_null() {
    assert_eq!(
        run_bootstrap_int_completion(
            "AccessControllerDoPrivilegedTest.class",
            "privilegedActionReturnsNull",
            "()I"
        ),
        1
    );
}

#[test]
fn access_controller_privileged_action_propagates_unchecked() {
    assert_eq!(
        run_bootstrap_int_completion(
            "AccessControllerDoPrivilegedTest.class",
            "privilegedActionPropagatesUnchecked",
            "()I"
        ),
        1
    );
}

#[test]
fn access_controller_privileged_exception_action_returns_value() {
    assert_eq!(
        run_bootstrap_int_completion(
            "AccessControllerDoPrivilegedTest.class",
            "privilegedExceptionActionReturnsValue",
            "()I"
        ),
        1
    );
}

#[test]
fn access_controller_privileged_exception_action_wraps_checked_exception() {
    assert_eq!(
        run_bootstrap_int_completion(
            "AccessControllerDoPrivilegedTest.class",
            "privilegedExceptionActionWrapsCheckedException",
            "()I"
        ),
        1
    );
}

#[test]
fn access_controller_security_manager_remains_null_during_action() {
    assert_eq!(
        run_bootstrap_int_completion(
            "AccessControllerDoPrivilegedTest.class",
            "securityManagerRemainsNullDuringAction",
            "()I"
        ),
        1
    );
}

#[test]
fn uuid_from_string_round_trips_canonical_text() {
    assert_eq!(
        run_bootstrap_int_completion("UuidTest.class", "testFromStringRoundTrip", "()I"),
        1
    );
}

#[test]
fn uuid_from_string_rejects_invalid_text_with_illegal_argument_exception() {
    assert_eq!(
        run_bootstrap_int_completion(
            "UuidTest.class",
            "testInvalidFromStringThrowsIllegalArgumentException",
            "()I"
        ),
        1
    );
}

#[test]
fn uuid_constructor_preserves_most_significant_bits() {
    assert_eq!(
        run_bootstrap_int_completion(
            "UuidTest.class",
            "testConstructorPreservesMostSignificantBits",
            "()I"
        ),
        1
    );
}

#[test]
fn uuid_constructor_preserves_least_significant_bits() {
    assert_eq!(
        run_bootstrap_int_completion(
            "UuidTest.class",
            "testConstructorPreservesLeastSignificantBits",
            "()I"
        ),
        1
    );
}

#[test]
fn uuid_random_uuid_sets_version_and_variant_bits() {
    assert_eq!(
        run_bootstrap_int_completion("UuidTest.class", "testRandomUuidVersionAndVariant", "()I"),
        1
    );
}

#[test]
fn uuid_name_uuid_from_bytes_matches_hotspot_reference() {
    assert_eq!(
        run_bootstrap_int_completion(
            "UuidTest.class",
            "testNameUuidMatchesHotSpotReference",
            "()I"
        ),
        1
    );
}

#[test]
fn uuid_name_uuid_from_bytes_sets_version_and_variant_bits() {
    assert_eq!(
        run_bootstrap_int_completion("UuidTest.class", "testNameUuidVersionAndVariant", "()I"),
        1
    );
}

#[test]
fn uuid_equals_uses_128_bit_value() {
    assert_eq!(
        run_bootstrap_int_completion("UuidTest.class", "testEqualsUses128BitValue", "()I"),
        1
    );
}

#[test]
fn uuid_hash_code_uses_128_bit_value() {
    assert_eq!(
        run_bootstrap_int_completion("UuidTest.class", "testHashCodeUses128BitValue", "()I"),
        1
    );
}

#[test]
fn uuid_hashmap_key_lookup_uses_uuid_equality() {
    assert_eq!(
        run_bootstrap_int_completion(
            "UuidTest.class",
            "testHashMapKeyLookupUsesUuidEquality",
            "()I"
        ),
        1
    );
}

#[test]
fn uuid_hashset_membership_uses_uuid_equality() {
    assert_eq!(
        run_bootstrap_int_completion(
            "UuidTest.class",
            "testHashSetMembershipUsesUuidEquality",
            "()I"
        ),
        1
    );
}

#[test]
fn uuid_compare_to_orders_by_most_then_least_significant_bits() {
    assert_eq!(
        run_bootstrap_int_completion(
            "UuidTest.class",
            "testCompareToOrdersByMostThenLeastSignificantBits",
            "()I"
        ),
        1
    );
}

#[test]
fn uuid_fixture_runs_all_acceptance_criteria() {
    assert_eq!(
        run_bootstrap_int_completion("UuidTest.class", "runAll", "()I"),
        1
    );
}

#[test]
fn base64_fixture_runs() {
    assert_eq!(run_bootstrap_int("Base64Test.class", "runAll", "()I"), 1);
}

#[test]
fn jul_logger_basic_outputs_info_warning_and_drops_fine() {
    let (result, lines) =
        run_bootstrap_with_output("JulLoggerBasicTest.class", "run", "()I").unwrap();
    assert_eq!(result, Some(Slot::Int(1)));
    let output = lines.join("\n");
    assert!(output.contains("INFO:"));
    assert!(output.contains("WARNING:"));
    assert!(!output.contains("dropped"));
}

#[test]
fn jul_logger_get_logger_interns_by_name() {
    assert_eq!(
        run_bootstrap_int_completion("JulLoggerInterningTest.class", "sameNameIdentity", "()I"),
        1
    );
}

#[test]
fn jul_level_statics_and_parse_are_canonical() {
    assert_eq!(
        run_bootstrap_int_completion("JulLevelStaticsTest.class", "staticsAndParse", "()I"),
        1
    );
}

#[test]
fn jul_log_record_stores_core_fields() {
    assert_eq!(
        run_bootstrap_int_completion("JulLogRecordTest.class", "recordBasics", "()I"),
        1
    );
}

#[test]
fn jul_clinit_survives_logger_atomic_and_pattern_fields() {
    assert_eq!(
        run_bootstrap_int_completion("JulClinitSurvivalTest.class", "clinitSurvives", "()I"),
        1
    );
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
    ) -> Result<Option<Slot>> {
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
    ) -> Result<Option<Slot>> {
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
    assert!(matches!(err, Error::DivisionByZero));
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
        parse, {AttributeData, CpEntry},
    };

    let bytes = std::fs::read(fixture(class_name)).expect("fixture not found");
    let cf = parse(&bytes).expect("parse failed");

    let method = cf
        .methods
        .iter()
        .find(|m| {
            let Some(Some(CpEntry::Utf8(s))) = cf.constant_pool.get(m.name_index.0 as usize) else {
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
    // Register the JDK synthetics (java/lang/Object, java/lang/Record, the
    // exception hierarchy, ...) exactly as a real interpreter run does. Fixture
    // classes chain super-constructor calls (e.g. Object.<init>, Record.<init>,
    // RuntimeException.<init>) that must resolve; without bootstrap they now
    // surface as catchable NoSuchMethodError instead of the former lenient swallow.
    bootstrap_stdlib(&mut registry, &mut heap);
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
    assert!(matches!(err, Error::MethodNotFound { .. }));
}

#[test]
fn invokevirtual_missing_loaded_method_returns_method_not_found() {
    use duke_classfile::CpIndex;
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
        is_native: false,
        is_abstract: false,
        instructions: Arc::clone(&instructions),
        max_stack: 1,
        max_locals: 1,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (1, 1), (4, 2)])),
        line_number_table: Vec::new(),
        source_file: None,
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
        load_source: ClassLoadSource::Classfile,
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
            Error::MethodNotFound { ref name, ref descriptor }
            if name == "java/lang/Class.missingMethod"
                && descriptor == "()Ljava/lang/Object;"
        ),
        "expected MethodNotFound for missing invokevirtual target, got {err:?}"
    );
}

/// Regression: `getClass()` invoked through an *interface-typed* callsite must
/// resolve the inherited `java/lang/Object.getClass` native by walking the
/// receiver's super chain. Real Spring Boot bytecode calls
/// `Configurator.getClass()` on a `DefaultJoranConfigurator` (which inherits
/// `getClass` from `Object` via an intermediate base class). Before the fix the
/// invokeinterface native fallback only probed the receiver class and the
/// interface directly — never the super chain — so `Object.getClass` was never
/// found and dispatch raised `MethodNotFound`.
#[test]
#[allow(clippy::too_many_lines)]
fn invokeinterface_getclass_resolves_inherited_object_native_via_super_chain() {
    use duke_classfile::CpIndex;
    use std::collections::HashMap;
    use std::sync::Arc;

    // Constant pool: an InterfaceMethodref for `Cfg.getClass()Ljava/lang/Class;`.
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
        Some(CpEntry::Utf8("Cfg".to_string())),
        Some(CpEntry::Utf8("getClass".to_string())),
        Some(CpEntry::Utf8("()Ljava/lang/Class;".to_string())),
    ]);
    let instructions: Arc<[(usize, Instruction)]> = vec![
        (0, Instruction::Aload0),
        (
            1,
            Instruction::Invokeinterface {
                index: CpIndex(1),
                count: 1,
            },
        ),
        (6, Instruction::Areturn),
    ]
    .into();
    let method = MethodEntry {
        name: "callGetClass".to_string(),
        descriptor: "(Ljava/lang/Object;)Ljava/lang/Object;".to_string(),
        is_public: true,
        is_static: true,
        is_native: false,
        is_abstract: false,
        instructions: Arc::clone(&instructions),
        max_stack: 1,
        max_locals: 1,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (1, 1), (6, 2)])),
        line_number_table: Vec::new(),
        source_file: None,
    };
    let caller_ctx = ClassContext {
        class_name: "TestCaller".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: cp,
        methods: vec![method],
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Classfile,
    };

    // Marker interface `Cfg` (declares no `getClass` of its own).
    let cfg_iface = ClassContext {
        class_name: "Cfg".to_string(),
        super_class: None,
        interfaces: Vec::new(),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    // Intermediate base class between the impl and Object — mirrors logback's
    // `ContextAwareBase` so the native only resolves by walking past it.
    let base_ctx = ClassContext {
        class_name: "CfgBase".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    // Concrete receiver: `CfgImpl extends CfgBase implements Cfg`, no own methods.
    let impl_ctx = ClassContext {
        class_name: "CfgImpl".to_string(),
        super_class: Some("CfgBase".to_string()),
        interfaces: vec!["Cfg".to_string()],
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.register(caller_ctx);
    registry.register(cfg_iface);
    registry.register(base_ctx);
    registry.register(impl_ctx);
    let loader = fixtures_loader();

    let impl_ref = heap.allocate("CfgImpl".to_string(), 0);
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "TestCaller",
        "callGetClass",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        &[Slot::Reference(Some(impl_ref))],
    )
    .expect("invokeinterface getClass should resolve the inherited Object native")
    .expect("getClass should return a Class object");

    let Slot::Reference(Some(class_ref)) = result else {
        panic!("expected a Class reference from getClass, got {result:?}");
    };
    assert_eq!(
        class_internal_name_from_ref(&heap, class_ref).unwrap(),
        "CfgImpl",
        "getClass() must report the receiver's concrete runtime class"
    );
}

/// The minimal synthetic `java/lang/ref/WeakReference` round-trips its referent:
/// `new WeakReference(x)` then `.get()` returns `x`. Mirrors commons-logging's
/// `thisClassLoaderRef` (a static `WeakReference<ClassLoader>` it constructs once
/// and only ever reads back). `get()` resolves on the `Reference` base via the
/// receiver's super chain. This is a NON-COLLECTING strong-ref-backed stub.
#[test]
fn weak_reference_get_round_trips_referent() {
    use duke_classfile::CpIndex;
    use std::collections::HashMap;
    use std::sync::Arc;

    // CP: new/init/get for java/lang/ref/WeakReference.
    let cp = make_cp(vec![
        Some(CpEntry::Class {
            name_index: CpIndex(2),
        }),
        Some(CpEntry::Utf8("java/lang/ref/WeakReference".to_string())),
        Some(CpEntry::Methodref {
            class_index: CpIndex(1),
            name_and_type_index: CpIndex(4),
        }),
        Some(CpEntry::NameAndType {
            name_index: CpIndex(5),
            descriptor_index: CpIndex(6),
        }),
        Some(CpEntry::Utf8("<init>".to_string())),
        Some(CpEntry::Utf8("(Ljava/lang/Object;)V".to_string())),
        Some(CpEntry::Methodref {
            class_index: CpIndex(1),
            name_and_type_index: CpIndex(8),
        }),
        Some(CpEntry::NameAndType {
            name_index: CpIndex(9),
            descriptor_index: CpIndex(10),
        }),
        Some(CpEntry::Utf8("get".to_string())),
        Some(CpEntry::Utf8("()Ljava/lang/Object;".to_string())),
    ]);
    let instructions: Arc<[(usize, Instruction)]> = vec![
        (0, Instruction::New(CpIndex(1))),
        (3, Instruction::Dup),
        (4, Instruction::Aload0), // referent (local 0)
        (5, Instruction::Invokespecial(CpIndex(3))),
        (8, Instruction::Invokevirtual(CpIndex(7))),
        (11, Instruction::Areturn),
    ]
    .into();
    let method = MethodEntry {
        name: "roundTrip".to_string(),
        descriptor: "(Ljava/lang/Object;)Ljava/lang/Object;".to_string(),
        is_public: true,
        is_static: true,
        is_native: false,
        is_abstract: false,
        instructions: Arc::clone(&instructions),
        max_stack: 3,
        max_locals: 1,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([
            (0, 0),
            (3, 1),
            (4, 2),
            (5, 3),
            (8, 4),
            (11, 5),
        ])),
        line_number_table: Vec::new(),
        source_file: None,
    };
    let caller_ctx = ClassContext {
        class_name: "TestWeakRef".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: cp,
        methods: vec![method],
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Classfile,
    };

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.register(caller_ctx);
    let loader = fixtures_loader();

    let referent = heap.allocate("java/lang/Object".to_string(), 0);
    let mut sink: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "TestWeakRef",
        "roundTrip",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        &[Slot::Reference(Some(referent))],
    )
    .expect("WeakReference new/init/get should execute")
    .expect("WeakReference.get should return the referent");

    assert_eq!(
        result,
        Slot::Reference(Some(referent)),
        "WeakReference.get() must return the exact referent stored at construction"
    );
}

#[test]
fn object_constructor_dispatches_via_invokespecial() {
    use duke_classfile::CpIndex;
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
        is_native: false,
        is_abstract: false,
        instructions: Arc::clone(&target_ctor_instructions),
        max_stack: 1,
        max_locals: 1,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (1, 1), (4, 2)])),
        line_number_table: Vec::new(),
        source_file: None,
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
        load_source: ClassLoadSource::Classfile,
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
    use duke_classfile::CpIndex;
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
        is_native: false,
        is_abstract: false,
        instructions: Arc::clone(&caller_instructions),
        max_stack: 1,
        max_locals: 1,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (1, 1), (4, 2)])),
        line_number_table: Vec::new(),
        source_file: None,
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
        load_source: ClassLoadSource::Classfile,
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
        load_source: ClassLoadSource::Classfile,
    };
    let child_instructions: Arc<[(usize, Instruction)]> =
        vec![(0, Instruction::Bipush(7)), (2, Instruction::Ireturn)].into();
    let child_method = MethodEntry {
        name: "value".to_string(),
        descriptor: "()I".to_string(),
        is_public: true,
        is_static: false,
        is_native: false,
        is_abstract: false,
        instructions: Arc::clone(&child_instructions),
        max_stack: 1,
        max_locals: 1,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (2, 1)])),
        line_number_table: Vec::new(),
        source_file: None,
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
        load_source: ClassLoadSource::Classfile,
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
    use duke_classfile::CpIndex;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
    fn native_value(
        _args: &[Slot],
        _heap: &mut duke_gc::Heap,
        _out: &mut dyn Write,
        _control: &mut NativeControl,
    ) -> Result<Option<Slot>> {
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
        is_native: false,
        is_abstract: false,
        instructions: Arc::clone(&caller_instructions),
        max_stack: 1,
        max_locals: 1,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (1, 1), (4, 2)])),
        line_number_table: Vec::new(),
        source_file: None,
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
        load_source: ClassLoadSource::Classfile,
    };
    let target_instructions: Arc<[(usize, Instruction)]> =
        vec![(0, Instruction::Bipush(7)), (2, Instruction::Ireturn)].into();
    let target_method = MethodEntry {
        name: "value".to_string(),
        descriptor: "()I".to_string(),
        is_public: true,
        is_static: false,
        is_native: false,
        is_abstract: false,
        instructions: Arc::clone(&target_instructions),
        max_stack: 1,
        max_locals: 1,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (2, 1)])),
        line_number_table: Vec::new(),
        source_file: None,
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
        load_source: ClassLoadSource::Classfile,
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
    use duke_classfile::CpIndex;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[allow(clippy::unnecessary_wraps)] // must match CallbackNativeHandler signature
    fn native_value(
        _args: &[Slot],
        _heap: &mut duke_gc::Heap,
        _out: &mut dyn Write,
        _control: &mut NativeControl,
        _ops: &mut dyn CallbackOps,
    ) -> Result<Option<Slot>> {
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
        is_native: false,
        is_abstract: false,
        instructions: Arc::clone(&caller_instructions),
        max_stack: 1,
        max_locals: 0,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (3, 1)])),
        line_number_table: Vec::new(),
        source_file: None,
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
        load_source: ClassLoadSource::Classfile,
    };
    let target_instructions: Arc<[(usize, Instruction)]> =
        vec![(0, Instruction::Bipush(7)), (2, Instruction::Ireturn)].into();
    let target_method = MethodEntry {
        name: "value".to_string(),
        descriptor: "()I".to_string(),
        is_public: true,
        is_static: true,
        is_native: false,
        is_abstract: false,
        instructions: Arc::clone(&target_instructions),
        max_stack: 1,
        max_locals: 0,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (2, 1)])),
        line_number_table: Vec::new(),
        source_file: None,
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
        load_source: ClassLoadSource::Classfile,
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

/// Regression: an `invokestatic` whose `Methodref` is symbolically bound to a
/// subclass must resolve a `public static` method declared on a *superclass*
/// (JVMS §5.4.3.3). Mirror of the real commons-logging bug where
/// `LogFactoryImpl.objectId(...)` is invoked via a Methodref bound to the
/// subclass `LogFactoryImpl`, but the body is declared on the abstract
/// superclass `LogFactory`. Before the fix the invokestatic slow-path resolver
/// flat-scanned only the referenced class, found nothing, and raised
/// `MethodNotFound`. The resolver now walks the super chain and rebinds the
/// callee class to where the body was found.
#[test]
#[allow(clippy::too_many_lines)]
fn invokestatic_resolves_inherited_static_method() {
    use duke_classfile::CpIndex;
    use std::collections::HashMap;
    use std::sync::Arc;

    // Caller: `Invokestatic` on `SubStaticImpl.inheritedStatic()I`. The
    // Methodref is bound to the *subclass*, which declares no such method.
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
        Some(CpEntry::Utf8("SubStaticImpl".to_string())),
        Some(CpEntry::Utf8("inheritedStatic".to_string())),
        Some(CpEntry::Utf8("()I".to_string())),
    ]);
    let caller_instructions: Arc<[(usize, Instruction)]> = vec![
        (0, Instruction::Invokestatic(CpIndex(1))),
        (3, Instruction::Ireturn),
    ]
    .into();
    let caller_method = MethodEntry {
        name: "callInherited".to_string(),
        descriptor: "()I".to_string(),
        is_public: true,
        is_static: true,
        is_native: false,
        is_abstract: false,
        instructions: Arc::clone(&caller_instructions),
        max_stack: 1,
        max_locals: 0,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (3, 1)])),
        line_number_table: Vec::new(),
        source_file: None,
    };
    let caller_ctx = ClassContext {
        class_name: "InheritedStaticCaller".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: caller_cp,
        methods: vec![caller_method],
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Classfile,
    };

    // Superclass declares the `public static` body (returns 7).
    let super_instructions: Arc<[(usize, Instruction)]> =
        vec![(0, Instruction::Bipush(7)), (2, Instruction::Ireturn)].into();
    let super_method = MethodEntry {
        name: "inheritedStatic".to_string(),
        descriptor: "()I".to_string(),
        is_public: true,
        is_static: true,
        is_native: false,
        is_abstract: false,
        instructions: Arc::clone(&super_instructions),
        max_stack: 1,
        max_locals: 0,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (2, 1)])),
        line_number_table: Vec::new(),
        source_file: None,
    };
    let super_ctx = ClassContext {
        class_name: "SuperStaticBase".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: Vec::new(),
        methods: vec![super_method],
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Classfile,
    };

    // Subclass declares NO `inheritedStatic` of its own — it inherits the static.
    let sub_ctx = ClassContext {
        class_name: "SubStaticImpl".to_string(),
        super_class: Some("SuperStaticBase".to_string()),
        interfaces: Vec::new(),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Classfile,
    };

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.register(caller_ctx);
    registry.register(super_ctx);
    registry.register(sub_ctx);
    let loader = make_simple_loader();
    let mut sink: Vec<u8> = Vec::new();

    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "InheritedStaticCaller",
        "callInherited",
        "()I",
        &[],
    )
    .expect("invokestatic should resolve the inherited static via the super chain");

    assert_eq!(result, Some(Slot::Int(7)));
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
    use duke_classfile::CpIndex;
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
    assert!(matches!(err, Error::InvalidMethodref { index: 99 }));
}

#[test]
fn resolve_methodref_not_a_methodref() {
    let cp = make_cp(vec![Some(CpEntry::Utf8("not a methodref".to_string()))]);
    let err = resolve_methodref(&cp, 1).unwrap_err();
    assert!(matches!(err, Error::InvalidMethodref { .. }));
}

#[test]
fn build_class_context_initializes_static_defaults_by_descriptor() {
    use duke_classfile::{ClassAccessFlags, FieldAccessFlags};
    use duke_classfile::{ClassFile, CpIndex, FieldInfo};

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
    use duke_classfile::CpIndex;
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
        is_native: false,
        is_abstract: false,
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
        line_number_table: Vec::new(),
        source_file: None,
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
        load_source: ClassLoadSource::Classfile,
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
    use duke_classfile::CpIndex;
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
    assert!(matches!(err, Error::InvalidFieldref { .. }));
}

// ---- Phase 7: Arrays ----

fn run_class_long(class_name: &str, method_name: &str, descriptor: &str, args: Vec<i32>) -> i64 {
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

fn run_class_double(class_name: &str, method_name: &str, descriptor: &str, args: Vec<i32>) -> f64 {
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
        Error::ArrayIndexOutOfBounds {
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
        Error::TypeMismatch {
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
    assert!(matches!(err, Error::NegativeArraySize { size: -1 }));
}

#[test]
fn athrow_propagates_as_java_exception() {
    use duke_bytecode::Instruction::*;
    let instrs = vec![(0, Iconst1), (1, Newarray(ArrayType::Int)), (3, Athrow)];
    let err = execute(&instrs, &[], vec![], 2, 0).unwrap_err();
    assert!(matches!(err, Error::JavaException { .. }));
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
    assert!(matches!(err, Error::JavaException { .. }));
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
    // See run_class_int: register JDK synthetics so super-constructor calls resolve.
    bootstrap_stdlib(&mut registry, &mut heap);
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
    ) -> Result<Option<Slot>> {
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

// ---- Category-2 (long/double) aware dup2 / dup2_x1 / dup2_x2 (JVMS §6.5) ----
// Regression coverage for the pre-existing bug where Dup2/Dup2X1/Dup2X2
// unconditionally duplicated two operand-stack cells. In Duke a long/double is a
// SINGLE Slot, so the Form-2/Form-4 (category-2) cases must duplicate one slot.
// With the old code these sequences underflowed the stack (popping a phantom
// second cell) and corrupted values (e.g. CHM addCount losing `this`).

#[test]
fn stack_ops_dup2_category2_long_duplicates_single_slot() {
    // dup2 Form 2: a single category-2 value is duplicated as one slot.
    // Old (buggy) code popped a second, non-existent cell → underflow error.
    let instrs = vec![
        (0, Instruction::Bipush(7)),
        (1, Instruction::I2l),  // 7L
        (2, Instruction::Dup2), // → 7L, 7L
        (3, Instruction::Ladd), // → 14L
        (4, Instruction::Lreturn),
    ];
    let r = execute(&instrs, &[], vec![], 10, 2).unwrap();
    assert_eq!(r, Some(Slot::Long(14)));
}

#[test]
fn stack_ops_dup2_category1_two_ints_still_duplicates_both() {
    // dup2 Form 1 is unchanged: two category-1 values are both duplicated.
    let instrs = vec![
        (0, Instruction::Bipush(3)),
        (1, Instruction::Bipush(5)),
        (2, Instruction::Dup2), // → 3, 5, 3, 5
        (3, Instruction::Iadd),
        (4, Instruction::Iadd),
        (5, Instruction::Iadd), // 3+5+3+5 = 16
        (6, Instruction::Ireturn),
    ];
    let r = execute(&instrs, &[], vec![], 10, 2).unwrap();
    assert_eq!(r, Some(Slot::Int(16)));
}

#[test]
fn stack_ops_dup2_x1_category2_long_over_int() {
    // dup2_x1 Form 2: value1 = long (cat 2), value2 = int (cat 1).
    // ..., int, long → ..., long, int, long
    let instrs = vec![
        (0, Instruction::Bipush(3)), // int 3 (value2)
        (1, Instruction::Bipush(7)),
        (2, Instruction::I2l),    // 7L (value1); stack: [3, 7L]
        (3, Instruction::Dup2X1), // → [7L, 3, 7L]
        (4, Instruction::L2i),    // top 7L → 7; [7L, 3, 7]
        (5, Instruction::Iadd),   // 3+7 = 10; [7L, 10]
        (6, Instruction::I2l),    // [7L, 10L]
        (7, Instruction::Ladd),   // 7+10 = 17L
        (8, Instruction::Lreturn),
    ];
    let r = execute(&instrs, &[], vec![], 10, 2).unwrap();
    assert_eq!(r, Some(Slot::Long(17)));
}

#[test]
fn stack_ops_dup2_x2_form4_two_longs() {
    // dup2_x2 Form 4: value1, value2 both category 2.
    // ..., bL, aL → ..., aL, bL, aL
    let instrs = vec![
        (0, Instruction::Bipush(4)),
        (1, Instruction::I2l), // 4L (value2, bottom)
        (2, Instruction::Bipush(9)),
        (3, Instruction::I2l),    // 9L (value1, top); stack: [4L, 9L]
        (4, Instruction::Dup2X2), // → [9L, 4L, 9L]
        (5, Instruction::Ladd),   // 4+9 = 13L; [9L, 13L]
        (6, Instruction::Ladd),   // 9+13 = 22L
        (7, Instruction::Lreturn),
    ];
    let r = execute(&instrs, &[], vec![], 10, 2).unwrap();
    assert_eq!(r, Some(Slot::Long(22)));
}

#[test]
fn stack_ops_dup2_x2_form2_long_over_two_ints() {
    // dup2_x2 Form 2: value1 = long (cat 2); value2, value3 = int (cat 1).
    // ..., i3, i2, longV1 → ..., longV1, i3, i2, longV1
    let instrs = vec![
        (0, Instruction::Bipush(3)), // int 3 (value3, bottom)
        (1, Instruction::Bipush(5)), // int 5 (value2)
        (2, Instruction::Bipush(7)),
        (3, Instruction::I2l),    // 7L (value1, top); stack: [3, 5, 7L]
        (4, Instruction::Dup2X2), // → [7L, 3, 5, 7L]
        (5, Instruction::L2i),    // top 7L → 7; [7L, 3, 5, 7]
        (6, Instruction::Iadd),   // 5+7 = 12; [7L, 3, 12]
        (7, Instruction::Iadd),   // 3+12 = 15; [7L, 15]
        (8, Instruction::I2l),    // [7L, 15L]
        (9, Instruction::Ladd),   // 7+15 = 22L
        (10, Instruction::Lreturn),
    ];
    let r = execute(&instrs, &[], vec![], 10, 2).unwrap();
    assert_eq!(r, Some(Slot::Long(22)));
}

#[test]
fn stack_ops_dup2_x2_form3_two_ints_over_long() {
    // dup2_x2 Form 3: value1, value2 = int (cat 1); value3 = long (cat 2).
    // ..., longV3, i2, i1 → ..., i2, i1, longV3, i2, i1
    let instrs = vec![
        (0, Instruction::Bipush(6)),
        (1, Instruction::I2l),       // 6L (value3, bottom)
        (2, Instruction::Bipush(3)), // int 3 (value2)
        (3, Instruction::Bipush(5)), // int 5 (value1, top); stack: [6L, 3, 5]
        (4, Instruction::Dup2X2),    // → [3, 5, 6L, 3, 5]
        (5, Instruction::Iadd),      // 3+5 = 8; [3, 5, 6L, 8]
        (6, Instruction::I2l),       // [3, 5, 6L, 8L]
        (7, Instruction::Ladd),      // 6+8 = 14L; [3, 5, 14L]
        (8, Instruction::L2i),       // [3, 5, 14]
        (9, Instruction::Iadd),      // 5+14 = 19; [3, 19]
        (10, Instruction::Iadd),     // 3+19 = 22
        (11, Instruction::Ireturn),
    ];
    let r = execute(&instrs, &[], vec![], 10, 2).unwrap();
    assert_eq!(r, Some(Slot::Int(22)));
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
        Err(Error::TypeMismatch {
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
        load_source: ClassLoadSource::Classfile,
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

// ---- Issue #854: String.equalsIgnoreCase fixture ----

#[test]
fn string_equals_ignore_case_fixture_prints_ok() {
    let mut registry = ClassRegistry::new();
    registry.register(load_class("StringEqualsIgnoreCase.class"));
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();

    // Build empty String[] array on the heap for main.
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 0);
    let main_args = vec![Slot::Reference(Some(arr_ref))];

    let mut out: Vec<u8> = Vec::new();
    let result = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut out,
        "StringEqualsIgnoreCase",
        "main",
        "([Ljava/lang/String;)V",
        &main_args,
    )
    .unwrap();
    assert_eq!(result, None);
    assert_eq!(String::from_utf8_lossy(&out).trim(), "OK");
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
    let bytes = std::fs::read(fixture("StringConcatTest.class")).expect("StringConcatTest.class");
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

fn run_jar_fixture_int(
    entry_class: &str,
    jar_name: &str,
    method_name: &str,
    descriptor: &str,
) -> i32 {
    let jar_path = fixtures_dir().join(jar_name);
    let loader = duke_loader::ZipLoader::open(&jar_path).expect("open jar fixture");
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry
        .ensure_loaded(entry_class, &loader)
        .expect("load jar fixture entry class");
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut out,
        entry_class,
        method_name,
        descriptor,
        &[],
    )
    .expect("jar fixture should execute");
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
) -> Result<Option<Slot>> {
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
) -> Result<(Option<Slot>, Vec<String>)> {
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

fn run_service_loader_jar_int(jar_name: &str, class_name: &str, method_name: &str) -> i32 {
    let jar_path = fixture(jar_name);
    let loader = duke_loader::ZipLoader::open(&jar_path).expect("open service loader fixture jar");
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    assert!(
        registry
            .ensure_loaded(class_name, &loader)
            .expect("load service loader fixture class"),
        "fixture class {class_name} should be present in {jar_name}"
    );
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut out,
        class_name,
        method_name,
        "()I",
        &[],
    )
    .expect("service loader fixture should execute");
    match result {
        Some(Slot::Int(value)) => value,
        other => panic!("expected int result, got {other:?}"),
    }
}

macro_rules! charset_fixture_int_test {
    ($test_name:ident, $class_file:literal, $method_name:literal, $expected:expr) => {
        #[test]
        fn $test_name() {
            assert_eq!(
                run_bootstrap_int_completion($class_file, $method_name, "()I"),
                $expected
            );
        }
    };
}

charset_fixture_int_test!(
    charset_for_name_utf8_name,
    "CharsetForNameTest.class",
    "utf8Name",
    1
);
charset_fixture_int_test!(
    charset_for_name_utf16_name,
    "CharsetForNameTest.class",
    "utf16Name",
    1
);
charset_fixture_int_test!(
    charset_for_name_utf16be_name,
    "CharsetForNameTest.class",
    "utf16beName",
    1
);
charset_fixture_int_test!(
    charset_for_name_utf16le_name,
    "CharsetForNameTest.class",
    "utf16leName",
    1
);
charset_fixture_int_test!(
    charset_for_name_ascii_name,
    "CharsetForNameTest.class",
    "asciiName",
    1
);
charset_fixture_int_test!(
    charset_for_name_latin1_name,
    "CharsetForNameTest.class",
    "latin1Name",
    1
);
charset_fixture_int_test!(
    charset_aliases_resolve_to_canonical_instances,
    "CharsetForNameTest.class",
    "aliasesResolveToCanonicalInstances",
    1
);
charset_fixture_int_test!(
    charset_default_charset_is_utf8,
    "CharsetForNameTest.class",
    "defaultCharsetIsUtf8",
    1
);
charset_fixture_int_test!(
    charset_for_name_unknown_throws_unsupported_charset,
    "CharsetForNameTest.class",
    "unsupportedCharsetThrows",
    1
);
charset_fixture_int_test!(
    standard_charsets_use_canonical_cache,
    "CharsetForNameTest.class",
    "standardCharsetsUseCanonicalCache",
    1
);
charset_fixture_int_test!(
    charset_display_name_and_to_string_match_name,
    "CharsetForNameTest.class",
    "displayNameAndToStringMatchName",
    1
);
charset_fixture_int_test!(
    charset_equals_hash_code_and_registered,
    "CharsetForNameTest.class",
    "equalsHashCodeAndRegistered",
    1
);
charset_fixture_int_test!(
    string_bytes_round_trip_utf8_matrix,
    "StringBytesRoundTripTest.class",
    "utf8Matrix",
    1
);
charset_fixture_int_test!(
    string_bytes_round_trip_utf16_matrix,
    "StringBytesRoundTripTest.class",
    "utf16Matrix",
    1
);
charset_fixture_int_test!(
    string_bytes_round_trip_utf16be_matrix,
    "StringBytesRoundTripTest.class",
    "utf16beMatrix",
    1
);
charset_fixture_int_test!(
    string_bytes_round_trip_utf16le_matrix,
    "StringBytesRoundTripTest.class",
    "utf16leMatrix",
    1
);
charset_fixture_int_test!(
    string_bytes_round_trip_latin1_in_range,
    "StringBytesRoundTripTest.class",
    "latin1InRangeMatrix",
    1
);
charset_fixture_int_test!(
    string_bytes_round_trip_ascii_in_range,
    "StringBytesRoundTripTest.class",
    "asciiInRangeMatrix",
    1
);
charset_fixture_int_test!(
    string_bytes_lossy_charsets_use_question_mark,
    "StringBytesRoundTripTest.class",
    "lossyCharsetsUseQuestionMark",
    1
);
charset_fixture_int_test!(
    string_bytes_known_encoded_lengths,
    "StringBytesRoundTripTest.class",
    "knownEncodedLengths",
    1
);
charset_fixture_int_test!(
    string_bytes_default_offset_window,
    "StringBytesRoundTripTest.class",
    "defaultOffsetWindow",
    1
);
charset_fixture_int_test!(
    string_bytes_direct_charset_offset_window,
    "StringBytesRoundTripTest.class",
    "directCharsetOffsetWindow",
    1
);
charset_fixture_int_test!(
    string_bytes_named_get_bytes_utf8,
    "StringBytesNamedCharsetTest.class",
    "getBytesUtf8ByName",
    1
);
charset_fixture_int_test!(
    string_bytes_named_new_string_iso,
    "StringBytesNamedCharsetTest.class",
    "newStringIsoByName",
    1
);
charset_fixture_int_test!(
    string_bytes_default_bytes_use_utf8,
    "StringBytesNamedCharsetTest.class",
    "defaultBytesUseUtf8",
    1
);
charset_fixture_int_test!(
    string_bytes_named_offset_window,
    "StringBytesNamedCharsetTest.class",
    "namedOffsetWindow",
    1
);
charset_fixture_int_test!(
    string_bytes_named_get_bytes_unknown_throws_unsupported_encoding,
    "StringBytesNamedCharsetTest.class",
    "getBytesUnknownThrowsUnsupportedEncoding",
    1
);
charset_fixture_int_test!(
    string_bytes_named_constructor_unknown_throws_unsupported_encoding,
    "StringBytesNamedCharsetTest.class",
    "constructorUnknownThrowsUnsupportedEncoding",
    1
);
charset_fixture_int_test!(
    string_bytes_named_aliases_work,
    "StringBytesNamedCharsetTest.class",
    "aliasesWorkForNamedStringOverloads",
    1
);
charset_fixture_int_test!(
    charset_malformed_input_replaces_invalid_utf8,
    "CharsetMalformedInputTest.class",
    "invalidUtf8ReplacesMalformedBytes",
    1
);
charset_fixture_int_test!(
    charset_clinit_interop_header_length,
    "CharsetClinitInteropTest.class",
    "headerLength",
    11
);
charset_fixture_int_test!(
    charset_clinit_interop_protocol_name,
    "CharsetClinitInteropTest.class",
    "protocolName",
    1
);

#[test]
fn charset_clinit_interop_main_prints_static_values() {
    let (_, lines) = run_bootstrap_with_output(
        "CharsetClinitInteropTest.class",
        "main",
        "([Ljava/lang/String;)V",
    )
    .expect("clinit interop main should execute");
    assert_eq!(lines, vec!["11".to_string(), "UTF-8".to_string()]);
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

#[test]
fn service_loader_basic_fixture_discovers_providers_in_order() {
    assert_eq!(
        run_service_loader_jar_int(
            "service-loader-basic.jar",
            "ServiceLoaderBasicTest",
            "providersInDeclarationOrder",
        ),
        1
    );
}

#[test]
fn service_loader_comments_fixture_ignores_comments_and_blanks() {
    assert_eq!(
        run_service_loader_jar_int(
            "service-loader-comments.jar",
            "ServiceLoaderCommentsTest",
            "ignoresCommentsAndBlankLines",
        ),
        1
    );
}

#[test]
fn service_loader_malformed_fixture_throws_configuration_error_and_continues() {
    assert_eq!(
        run_service_loader_jar_int(
            "service-loader-malformed.jar",
            "ServiceLoaderMalformedTest",
            "missingProviderThrowsButIteratorContinues",
        ),
        1
    );
}

#[test]
fn service_loader_jdbc_smoke_fixture_instantiates_driver() {
    assert_eq!(
        run_service_loader_jar_int(
            "service-loader-jdbc-smoke.jar",
            "ServiceLoaderJdbcSmokeTest",
            "miniDriverLoadsAndResponds",
        ),
        1
    );
}

#[test]
fn atomic_integer_basic_fixture_exercises_core_surface() {
    assert_eq!(
        run_bootstrap_int("AtomicIntegerBasicTest.class", "basicOperations", "()I"),
        1
    );
}

#[test]
fn atomic_long_basic_fixture_preserves_wide_payloads() {
    assert_eq!(
        run_bootstrap_int("AtomicLongBasicTest.class", "basicOperations", "()I"),
        1
    );
}

#[test]
fn atomic_reference_basic_fixture_uses_identity_cas() {
    assert_eq!(
        run_bootstrap_int("AtomicReferenceBasicTest.class", "basicOperations", "()I"),
        1
    );
}

#[test]
fn atomic_boolean_basic_fixture_exercises_core_surface() {
    assert_eq!(
        run_bootstrap_int("AtomicBooleanBasicTest.class", "basicOperations", "()I"),
        1
    );
}

#[test]
fn atomic_contended_fixture_is_linearizable_under_threading_runtime() {
    assert_eq!(
        run_bootstrap_int_completion(
            "AtomicContendedTest.class",
            "twoThreadsIncrementFiftyTrials",
            "()I",
        ),
        2000
    );
}

#[test]
fn reentrant_lock_reports_hold_count_and_owner_status() {
    assert_eq!(
        run_bootstrap_int_completion(
            "ReentrantLockBasicTest.class",
            "reentrantHoldCountAndStatus",
            "()I",
        ),
        1
    );
}

#[test]
fn reentrant_lock_try_lock_fails_while_other_thread_holds_lock() {
    assert_eq!(
        run_bootstrap_int_completion(
            "ReentrantLockBasicTest.class",
            "tryLockFailsWhileAnotherThreadHolds",
            "()I",
        ),
        1
    );
}

#[test]
fn reentrant_lock_unlock_by_non_owner_throws() {
    assert_eq!(
        run_bootstrap_int_completion(
            "ReentrantLockBasicTest.class",
            "unlockByNonOwnerThrows",
            "()I",
        ),
        1
    );
}

#[test]
fn condition_producer_consumer_preserves_sequence() {
    assert_eq!(
        run_bootstrap_int_completion(
            "ConditionProducerConsumerTest.class",
            "producerConsumerSequence",
            "()I",
        ),
        12345
    );
}

#[test]
fn condition_await_reacquires_lock_after_signal() {
    assert_eq!(
        run_bootstrap_int_completion(
            "ConditionProducerConsumerTest.class",
            "awaitReacquiresLockAfterSignal",
            "()I",
        ),
        1
    );
}

#[test]
fn condition_operations_require_lock_ownership() {
    assert_eq!(
        run_bootstrap_int_completion(
            "ConditionProducerConsumerTest.class",
            "conditionCallsRequireLockOwnership",
            "()I",
        ),
        1111
    );
}

#[test]
fn count_down_latch_basic_fixture_exercises_core_surface() {
    assert_eq!(
        run_bootstrap_int_completion("CountDownLatchBasicTest.class", "basicOperations", "()I"),
        1
    );
}

#[test]
fn count_down_latch_contended_fixture_releases_all_waiters() {
    assert_eq!(
        run_bootstrap_int_completion(
            "CountDownLatchContendedTest.class",
            "fiveWorkersCompleteTwentyFiveTrials",
            "()I",
        ),
        125
    );
}

#[test]
fn semaphore_basic_fixture_exercises_permits_and_timeout() {
    assert_eq!(
        run_bootstrap_int_completion("SemaphoreBasicTest.class", "basicOperations", "()I"),
        1
    );
}

#[test]
fn semaphore_contended_fixture_bounds_parallelism() {
    assert_eq!(
        run_bootstrap_int_completion(
            "SemaphoreContendedTest.class",
            "boundedCriticalSectionTenTrials",
            "()I",
        ),
        3
    );
}

#[test]
fn cyclic_barrier_basic_fixture_reuses_generations() {
    assert_eq!(
        run_bootstrap_int_completion(
            "CyclicBarrierBasicTest.class",
            "twoCyclesReturnAllArrivalIndexes",
            "()I",
        ),
        1
    );
}

#[test]
fn cyclic_barrier_action_fixture_runs_once_per_generation() {
    assert_eq!(
        run_bootstrap_int_completion(
            "CyclicBarrierActionTest.class",
            "actionRunsOncePerGeneration",
            "()I",
        ),
        4
    );
}

#[test]
fn sync_primitive_interop_fixture_composes_all_three() {
    assert_eq!(
        run_bootstrap_int_completion(
            "SyncPrimitiveInteropTest.class",
            "primitivesCoordinateTogether",
            "()I",
        ),
        1
    );
}

#[test]
fn sync_primitive_interrupt_fixture_latch_raises_interrupted_exception() {
    assert_eq!(
        run_bootstrap_int_completion(
            "SyncPrimitiveInterruptTest.class",
            "latchAwaitInterruptRaises",
            "()I",
        ),
        1
    );
}

#[test]
fn sync_primitive_interrupt_fixture_uninterruptible_acquire_preserves_flag() {
    assert_eq!(
        run_bootstrap_int_completion(
            "SyncPrimitiveInterruptTest.class",
            "semaphoreUninterruptiblePreservesFlag",
            "()I",
        ),
        1
    );
}

#[test]
fn sync_primitive_interrupt_fixture_barrier_breaks_generation() {
    assert_eq!(
        run_bootstrap_int_completion(
            "SyncPrimitiveInterruptTest.class",
            "barrierInterruptBreaksGeneration",
            "()I",
        ),
        110
    );
}

#[test]
fn read_write_lock_allows_overlapping_readers() {
    assert_eq!(
        run_bootstrap_int_completion("ReadWriteLockTest.class", "twoReadersMayOverlap", "()I"),
        1
    );
}

#[test]
fn read_write_lock_write_lock_excludes_readers_until_unlock() {
    assert_eq!(
        run_bootstrap_int_completion(
            "ReadWriteLockTest.class",
            "writeLockExcludesReadersUntilUnlock",
            "()I",
        ),
        3
    );
}

#[test]
fn read_write_lock_read_lock_excludes_writer_try_lock() {
    assert_eq!(
        run_bootstrap_int_completion(
            "ReadWriteLockTest.class",
            "readLockExcludesWriterTryLock",
            "()I",
        ),
        1
    );
}

#[test]
fn read_write_lock_supports_write_reentrancy_and_downgrade() {
    assert_eq!(
        run_bootstrap_int_completion(
            "ReadWriteLockTest.class",
            "writeReentrantAndDowngrade",
            "()I",
        ),
        1
    );
}

#[test]
fn read_write_lock_read_lock_new_condition_is_unsupported() {
    assert_eq!(
        run_bootstrap_int_completion(
            "ReadWriteLockTest.class",
            "readLockNewConditionUnsupported",
            "()I",
        ),
        1
    );
}

#[test]
fn read_write_lock_write_unlock_by_non_owner_throws() {
    assert_eq!(
        run_bootstrap_int_completion(
            "ReadWriteLockTest.class",
            "writeUnlockByNonOwnerThrows",
            "()I",
        ),
        1
    );
}

#[test]
fn executor_basic_fixture_runs_side_effect_and_awaits_termination() {
    let (result, lines) = run_bootstrap_with_output("ExecutorBasicTest.class", "run", "()V")
        .expect("executor basic fixture should execute");

    assert_eq!(result, None);
    assert_eq!(lines, vec!["1"]);
}

#[test]
fn executor_callable_future_get_returns_boxed_result() {
    let (result, lines) = run_bootstrap_with_output("ExecutorCallableTest.class", "run", "()V")
        .expect("executor callable fixture should execute");

    assert_eq!(result, None);
    assert_eq!(lines, vec!["42"]);
}

#[test]
fn executor_pool_fixture_runs_thousand_tasks() {
    let (result, lines) = run_bootstrap_with_output("ExecutorPoolTest.class", "run", "()V")
        .expect("executor pool fixture should execute");

    assert_eq!(result, None);
    assert_eq!(lines, vec!["1000"]);
}

#[test]
fn executor_timeout_fixture_reports_timeout() {
    let (result, lines) = run_bootstrap_with_output("ExecutorTimeoutTest.class", "run", "()V")
        .expect("executor timeout fixture should execute");

    assert_eq!(result, None);
    assert_eq!(lines, vec!["timeout"]);
}

#[test]
fn executor_cancel_fixture_reports_cancelled() {
    let (result, lines) = run_bootstrap_with_output("ExecutorCancelTest.class", "run", "()V")
        .expect("executor cancel fixture should execute");

    assert_eq!(result, None);
    assert_eq!(lines, vec!["cancelled"]);
}

#[test]
fn executor_cached_factory_works() {
    assert_eq!(
        run_bootstrap_int_completion("ExecutorCallableTest.class", "cachedFactoryWorks", "()I"),
        1
    );
}

#[test]
fn executor_submit_runnable_with_result_routes_correctly() {
    assert_eq!(
        run_bootstrap_int_completion(
            "ExecutorCallableTest.class",
            "runnableWithResultWorks",
            "()I",
        ),
        77
    );
}

#[test]
fn executor_execution_exception_wraps_original_cause() {
    assert_eq!(
        run_bootstrap_int_completion(
            "ExecutorCallableTest.class",
            "executionExceptionWrapsCause",
            "()I",
        ),
        1
    );
}

#[test]
fn timeunit_seconds_to_millis_converts() {
    assert_eq!(
        run_bootstrap_long("ExecutorCallableTest.class", "secondsToMillis", "()J"),
        2000
    );
}

#[test]
fn atomic_service_loader_interop_fixture_allows_atomic_clinit() {
    assert_eq!(
        run_service_loader_jar_int(
            "service-loader-atomic.jar",
            "AtomicServiceLoaderInteropTest",
            "providerClinitUsesAtomicLong",
        ),
        1
    );
}

#[test]
fn concurrent_hashmap_default_ctor_put_get_size() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapBasicTest.class",
            "defaultCtorPutGetSize",
            "()I"
        ),
        15
    );
}

#[test]
fn concurrent_hashmap_capacity_ctor() {
    assert_eq!(
        run_bootstrap_int("ConcurrentHashMapBasicTest.class", "capacityCtor", "()I"),
        7
    );
}

#[test]
fn concurrent_hashmap_capacity_load_ctor() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapBasicTest.class",
            "capacityLoadCtor",
            "()I"
        ),
        8
    );
}

#[test]
fn concurrent_hashmap_capacity_load_concurrency_ctor() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapBasicTest.class",
            "capacityLoadConcurrencyCtor",
            "()I"
        ),
        9
    );
}

#[test]
fn concurrent_hashmap_copy_ctor_copies_existing_map() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapBasicTest.class",
            "copyCtorCopiesExistingMap",
            "()I"
        ),
        32
    );
}

#[test]
fn concurrent_hashmap_put_all_copies_entries() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapBasicTest.class",
            "putAllCopiesEntries",
            "()I"
        ),
        5
    );
}

#[test]
fn concurrent_hashmap_map_interface_dispatches() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapBasicTest.class",
            "mapInterfaceDispatchesToConcurrentHashMap",
            "()I"
        ),
        42
    );
}

#[test]
fn concurrent_hashmap_map_interface_key_set_dispatches() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapBasicTest.class",
            "mapInterfaceKeySetDispatchesToConcurrentHashMap",
            "()I"
        ),
        1
    );
}

#[test]
fn concurrent_hashmap_concurrent_map_interface_dispatches() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapBasicTest.class",
            "concurrentMapInterfaceDispatchesToConcurrentHashMap",
            "()I"
        ),
        58
    );
}

#[test]
fn concurrent_hashmap_contains_key_true_false() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapBasicTest.class",
            "containsKeyTrueFalse",
            "()I"
        ),
        1
    );
}

#[test]
fn concurrent_hashmap_contains_value_true_false() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapBasicTest.class",
            "containsValueTrueFalse",
            "()I"
        ),
        1
    );
}

#[test]
fn concurrent_hashmap_get_or_default_hit_miss() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapBasicTest.class",
            "getOrDefaultHitMiss",
            "()I"
        ),
        45
    );
}

#[test]
fn concurrent_hashmap_remove_returns_prior_value() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapBasicTest.class",
            "removeReturnsPriorValue",
            "()I"
        ),
        12
    );
}

#[test]
fn concurrent_hashmap_conditional_remove_branches() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapBasicTest.class",
            "conditionalRemoveBranches",
            "()I"
        ),
        11
    );
}

#[test]
fn concurrent_hashmap_put_if_absent_branches() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapBasicTest.class",
            "putIfAbsentBranches",
            "()I"
        ),
        14
    );
}

#[test]
fn concurrent_hashmap_replace_value_branches() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapBasicTest.class",
            "replaceValueBranches",
            "()I"
        ),
        19
    );
}

#[test]
fn concurrent_hashmap_replace_cas_branches() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapBasicTest.class",
            "replaceCasBranches",
            "()I"
        ),
        18
    );
}

#[test]
fn concurrent_hashmap_clear_then_is_empty() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapBasicTest.class",
            "clearThenIsEmpty",
            "()I"
        ),
        1
    );
}

#[test]
fn concurrent_hashmap_snapshot_views() {
    assert_eq!(
        run_bootstrap_int("ConcurrentHashMapBasicTest.class", "snapshotViews", "()I"),
        12
    );
}

#[test]
fn concurrent_hashmap_compute_if_absent_miss_and_hit() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapComputeTest.class",
            "computeIfAbsentMissAndHit",
            "()I"
        ),
        1
    );
}

#[test]
fn concurrent_hashmap_compute_if_present_hit() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapComputeTest.class",
            "computeIfPresentHit",
            "()I"
        ),
        1
    );
}

#[test]
fn concurrent_hashmap_compute_if_present_miss() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapComputeTest.class",
            "computeIfPresentMiss",
            "()I"
        ),
        1
    );
}

#[test]
fn concurrent_hashmap_compute_updates_value() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapComputeTest.class",
            "computeUpdatesValue",
            "()I"
        ),
        1
    );
}

#[test]
fn concurrent_hashmap_merge_accumulates_count() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapComputeTest.class",
            "mergeAccumulatesCount",
            "()I"
        ),
        3
    );
}

#[test]
fn concurrent_hashmap_for_each_visits_snapshot() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapComputeTest.class",
            "forEachVisitsSnapshot",
            "()I"
        ),
        1
    );
}

#[test]
fn concurrent_hashmap_put_null_key_throws() {
    assert_eq!(
        run_bootstrap_int("ConcurrentHashMapNullTest.class", "putNullKeyThrows", "()I"),
        1
    );
}

#[test]
fn concurrent_hashmap_put_null_value_throws() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapNullTest.class",
            "putNullValueThrows",
            "()I"
        ),
        1
    );
}

#[test]
fn concurrent_hashmap_get_null_throws() {
    assert_eq!(
        run_bootstrap_int("ConcurrentHashMapNullTest.class", "getNullThrows", "()I"),
        1
    );
}

#[test]
fn concurrent_hashmap_contains_key_null_throws() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapNullTest.class",
            "containsKeyNullThrows",
            "()I"
        ),
        1
    );
}

#[test]
fn concurrent_hashmap_put_if_absent_null_value_throws() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapNullTest.class",
            "putIfAbsentNullValueThrows",
            "()I"
        ),
        1
    );
}

#[test]
fn concurrent_hashmap_merge_null_value_throws() {
    assert_eq!(
        run_bootstrap_int(
            "ConcurrentHashMapNullTest.class",
            "mergeNullValueThrows",
            "()I"
        ),
        1
    );
}

#[test]
fn concurrent_hashmap_contention_merge_is_linearizable() {
    assert_eq!(
        run_bootstrap_int_completion(
            "ConcurrentHashMapContentionTest.class",
            "twoThreadsMergeFiftyTrials",
            "()I",
        ),
        2000
    );
}

#[test]
fn concurrent_hashmap_static_init_cache_reads() {
    assert_eq!(
        run_bootstrap_int("ConcurrentHashMapStaticInitTest.class", "readCache", "()I"),
        126
    );
}

#[test]
fn concurrent_hashmap_static_init_prints_expected_values() {
    let result =
        run_bootstrap_with_output("ConcurrentHashMapStaticInitTest.class", "printCache", "()V");
    assert!(
        result.is_ok(),
        "expected ConcurrentHashMap static init print smoke to run; got {result:?}"
    );
    let (value, lines) = result.unwrap();
    assert_eq!(value, None);
    assert_eq!(lines, vec!["42 84"]);
}

#[test]
fn concurrent_hashmap_static_init_main_prints_expected_values() {
    let ctx = load_class_context("ConcurrentHashMapStaticInitTest.class");
    let entry_class = ctx.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    let args_ref = heap.allocate("[Ljava/lang/String;".to_string(), 0);
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut out,
        &entry_class,
        "main",
        "([Ljava/lang/String;)V",
        &[Slot::Reference(Some(args_ref))],
    );
    assert!(
        result.is_ok(),
        "expected ConcurrentHashMap static init main smoke to run; got {result:?}"
    );
    assert_eq!(result.unwrap(), None);
    let lines: Vec<String> = String::from_utf8(out)
        .expect("captured output is utf8")
        .lines()
        .map(std::string::ToString::to_string)
        .collect();
    assert_eq!(lines, vec!["42 84"]);
}

#[test]
fn service_loader_load_with_url_class_loader_reads_service_resources() {
    let jar_path = fixture("service-loader-basic.jar")
        .canonicalize()
        .expect("canonical service loader jar");
    let loader = duke_loader::ZipLoader::open(&jar_path).expect("open service loader fixture jar");
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let url_loader_ref = allocate_url_class_loader_for_path(&mut registry, &mut heap, &jar_path);
    let service_class_ref = load_class_via_url_class_loader(
        &loader,
        &mut registry,
        &mut heap,
        url_loader_ref,
        "com.example.Greeter",
    )
    .expect("URLClassLoader.loadClass should succeed")
    .expect("loadClass should return a Class");

    let service_loader_slot = {
        let mut ops = InterpreterCallbackOps {
            registry: &mut registry,
            loader: &loader,
        };
        native_service_loader_load_with_loader(
            &[service_class_ref, Slot::Reference(Some(url_loader_ref))],
            &mut heap,
            &mut Vec::new(),
            &mut NativeControl::default(),
            &mut ops,
        )
        .expect("ServiceLoader.load(Class, ClassLoader) should succeed")
        .expect("ServiceLoader.load should return an instance")
    };
    let Slot::Reference(Some(service_loader_ref)) = service_loader_slot else {
        panic!("expected ServiceLoader reference");
    };
    let iterator_slot = native_service_loader_iterator(
        &[Slot::Reference(Some(service_loader_ref))],
        &mut heap,
        &mut Vec::new(),
        &mut NativeControl::default(),
    )
    .expect("ServiceLoader.iterator should succeed")
    .expect("iterator should return a reference");
    let Slot::Reference(Some(iterator_ref)) = iterator_slot else {
        panic!("expected iterator reference");
    };
    let first_provider_slot = {
        let mut ops = InterpreterCallbackOps {
            registry: &mut registry,
            loader: &loader,
        };
        native_service_loader_iter_next(
            &[Slot::Reference(Some(iterator_ref))],
            &mut heap,
            &mut Vec::new(),
            &mut NativeControl::default(),
            &mut ops,
        )
        .expect("iterator.next should instantiate first provider")
        .expect("iterator.next should return a provider")
    };
    let Slot::Reference(Some(provider_ref)) = first_provider_slot else {
        panic!("expected provider reference");
    };
    let provider_class = heap
        .get(provider_ref)
        .expect("provider object")
        .class_name
        .clone();
    assert!(
        provider_class.starts_with("com/example/Hello\0loader:"),
        "expected provider from explicit URLClassLoader, got {provider_class:?}"
    );
}

#[test]
fn resource_loading_fixture_runs_directory_cases() {
    assert_eq!(
        run_bootstrap_int_completion("ResourceLoadingTest.class", "runAll", "()I"),
        1
    );
}

#[test]
fn url_connection_open_connection_reads_resource() {
    // URL.openConnection().getInputStream() reads the classpath resource
    // end-to-end through the synthetic java/net/URLConnection, matching the
    // exact 27-byte content of ResourceLoadingTestData.txt.
    assert_eq!(
        run_bootstrap_int_completion(
            "UrlConnectionProbe.class",
            "openConnectionReadsResource",
            "()I"
        ),
        0
    );
}

#[test]
fn url_connection_natives_dispatch_directly() {
    // Direct-dispatch coverage: mint a spec-backed URL, call openConnection,
    // then getInputStream on the resulting URLConnection, and assert the exact
    // resource bytes flow through the ResourceInputStream.
    let data_path = fixtures_dir().join("ResourceLoadingTestData.txt");
    let expected = std::fs::read(&data_path).expect("read fixture data");
    let spec = format!("file://{}", data_path.to_string_lossy());

    let mut heap = duke_gc::Heap::new();
    let url_ref = allocate_string_backed_object(&mut heap, "java/net/URL", spec.clone())
        .expect("mint URL object");

    // openConnection() returns a java/net/URLConnection carrying the same spec.
    let conn_slot = native_url_open_connection(
        &[Slot::Reference(Some(url_ref))],
        &mut heap,
        &mut std::io::sink(),
        &mut NativeControl::default(),
    )
    .expect("openConnection succeeds")
    .expect("openConnection returns a reference");
    let conn_ref = match conn_slot {
        Slot::Reference(Some(r)) => r,
        other => panic!("expected URLConnection reference, got {other:?}"),
    };
    assert_eq!(
        heap.get(conn_ref).unwrap().class_name,
        "java/net/URLConnection"
    );
    assert_eq!(
        string_backed_object_value(&heap, conn_ref).expect("connection spec"),
        spec
    );

    // setUseCaches(false) is a no-op that returns without a value.
    let use_caches = native_url_connection_set_use_caches(
        &[Slot::Reference(Some(conn_ref)), Slot::Int(0)],
        &mut heap,
        &mut std::io::sink(),
        &mut NativeControl::default(),
    )
    .expect("setUseCaches succeeds");
    assert!(use_caches.is_none());

    // getInputStream() returns a duke/io/ResourceInputStream over the bytes.
    let stream_slot = native_url_connection_get_input_stream(
        &[Slot::Reference(Some(conn_ref))],
        &mut heap,
        &mut std::io::sink(),
        &mut NativeControl::default(),
    )
    .expect("getInputStream succeeds")
    .expect("getInputStream returns a reference");
    let stream_ref = match stream_slot {
        Slot::Reference(Some(r)) => r,
        other => panic!("expected ResourceInputStream reference, got {other:?}"),
    };
    assert_eq!(
        heap.get(stream_ref).unwrap().class_name,
        "duke/io/ResourceInputStream"
    );
    let array_ref = match heap.get(stream_ref).unwrap().fields[0] {
        Slot::Reference(Some(r)) => r,
        other => panic!("expected byte array reference, got {other:?}"),
    };
    let actual: Vec<u8> = heap
        .get(array_ref)
        .unwrap()
        .fields
        .iter()
        .map(|slot| match slot {
            Slot::Int(byte) => u8::try_from(*byte & 0xFF).unwrap(),
            other => panic!("expected int byte slot, got {other:?}"),
        })
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn resource_loading_meta_inf_fixture_runs_from_directory_loader() {
    assert_eq!(
        run_bootstrap_int_completion("ResourceLoadingJarTest.class", "readMetaInfMessage", "()I",),
        0
    );
}

#[test]
fn resource_loading_meta_inf_fixture_runs_from_jar_loader() {
    assert_eq!(
        run_jar_fixture_int(
            "ResourceLoadingJarTest",
            "resource-loading.jar",
            "runAll",
            "()I"
        ),
        1
    );
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

// ---- EnumSet + enum-reflection tests ----
// Expected values verified against real `java` (JDK 21) golden run.

#[test]
fn enumset_of_contains() {
    assert_eq!(
        run_bootstrap_int("EnumSetTest.class", "testOfContains", "()I"),
        1
    );
}

#[test]
fn enumset_of_not_contains() {
    assert_eq!(
        run_bootstrap_int("EnumSetTest.class", "testOfNotContains", "()I"),
        0
    );
}

#[test]
fn enumset_none_of_size() {
    assert_eq!(
        run_bootstrap_int("EnumSetTest.class", "testNoneOfSize", "()I"),
        0
    );
}

#[test]
fn enumset_all_of_size() {
    assert_eq!(
        run_bootstrap_int("EnumSetTest.class", "testAllOfSize", "()I"),
        7
    );
}

#[test]
fn enumset_range_size() {
    assert_eq!(
        run_bootstrap_int("EnumSetTest.class", "testRangeSize", "()I"),
        3
    );
}

#[test]
fn enumset_add_remove() {
    assert_eq!(
        run_bootstrap_int("EnumSetTest.class", "testAddRemove", "()I"),
        3
    );
}

#[test]
fn enumset_iterator_count() {
    assert_eq!(
        run_bootstrap_int("EnumSetTest.class", "testIteratorCount", "()I"),
        3
    );
}

#[test]
fn enumset_class_is_enum() {
    assert_eq!(
        run_bootstrap_int("EnumSetTest.class", "testIsEnum", "()I"),
        1
    );
    assert_eq!(
        run_bootstrap_int("EnumSetTest.class", "testStringNotEnum", "()I"),
        0
    );
}

#[test]
fn enumset_get_enum_constants_len() {
    assert_eq!(
        run_bootstrap_int("EnumSetTest.class", "testGetEnumConstantsLen", "()I"),
        7
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

// ---- Phase 28: Extended Math (trig / transcendental) ----

#[test]
fn math_ext_sin() {
    assert_eq!(run_bootstrap_int("MathExtTest.class", "testSin", "()I"), 1);
}

#[test]
fn math_ext_cos() {
    assert_eq!(run_bootstrap_int("MathExtTest.class", "testCos", "()I"), 1);
}

#[test]
fn math_ext_tan() {
    assert_eq!(run_bootstrap_int("MathExtTest.class", "testTan", "()I"), 1);
}

#[test]
fn math_ext_atan2() {
    assert_eq!(
        run_bootstrap_int("MathExtTest.class", "testAtan2", "()I"),
        1
    );
}

#[test]
fn math_ext_log() {
    assert_eq!(run_bootstrap_int("MathExtTest.class", "testLog", "()I"), 1);
}

#[test]
fn math_ext_log10() {
    assert_eq!(
        run_bootstrap_int("MathExtTest.class", "testLog10", "()I"),
        3
    );
}

#[test]
fn math_ext_exp() {
    assert_eq!(run_bootstrap_int("MathExtTest.class", "testExp", "()I"), 1);
}

#[test]
fn math_ext_signum_positive() {
    assert_eq!(
        run_bootstrap_int("MathExtTest.class", "testSignumPositive", "()I"),
        1
    );
}

#[test]
fn math_ext_signum_negative() {
    assert_eq!(
        run_bootstrap_int("MathExtTest.class", "testSignumNegative", "()I"),
        -1
    );
}

#[test]
fn math_ext_to_radians() {
    assert_eq!(
        run_bootstrap_int("MathExtTest.class", "testToRadians", "()I"),
        1
    );
}

#[test]
fn math_ext_to_degrees() {
    assert_eq!(
        run_bootstrap_int("MathExtTest.class", "testToDegrees", "()I"),
        180
    );
}

#[test]
fn math_ext_cbrt() {
    assert_eq!(run_bootstrap_int("MathExtTest.class", "testCbrt", "()I"), 3);
}

#[test]
fn math_ext_hypot() {
    assert_eq!(
        run_bootstrap_int("MathExtTest.class", "testHypot", "()I"),
        5
    );
}

#[test]
fn math_ext_round_float() {
    assert_eq!(
        run_bootstrap_int("MathExtTest.class", "testRoundFloat", "()I"),
        3
    );
}

// ---- Phase 28: System.arraycopy ----

#[test]
fn system_arraycopy_int_full() {
    assert_eq!(
        run_bootstrap_int("SystemArraycopyTest.class", "testIntArrayCopy", "()I"),
        30
    );
}

#[test]
fn system_arraycopy_partial() {
    assert_eq!(
        run_bootstrap_int("SystemArraycopyTest.class", "testPartialCopy", "()I"),
        6
    );
}

#[test]
fn system_arraycopy_string_array() {
    assert_eq!(
        run_bootstrap_int("SystemArraycopyTest.class", "testStringArrayCopy", "()I"),
        1
    );
}

#[test]
fn system_arraycopy_overlap_safe() {
    assert_eq!(
        run_bootstrap_int("SystemArraycopyTest.class", "testOverlapSafe", "()I"),
        4
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

// ---- Phase 28: HashMap iteration (keySet / values / entrySet) ----

#[test]
fn hashmap_keyset_size() {
    assert_eq!(
        run_bootstrap_int("HashMapIterTest.class", "testKeySet", "()I"),
        3
    );
}

#[test]
fn hashmap_keyset_for_each() {
    assert_eq!(
        run_bootstrap_int("HashMapIterTest.class", "testKeySetContains", "()I"),
        2
    );
}

#[test]
fn hashmap_values_sum() {
    assert_eq!(
        run_bootstrap_int("HashMapIterTest.class", "testValues", "()I"),
        60
    );
}

#[test]
fn hashmap_entryset_sum_values() {
    assert_eq!(
        run_bootstrap_int("HashMapIterTest.class", "testEntrySet", "()I"),
        3
    );
}

#[test]
fn hashmap_entryset_getkey() {
    assert_eq!(
        run_bootstrap_int("HashMapIterTest.class", "testEntrySetKeys", "()I"),
        5
    );
}

// ---- Phase 29: ArrayList extended methods ----

#[test]
fn arraylist_remove_at() {
    assert_eq!(
        run_bootstrap_int("ArrayListExtTest.class", "testRemoveAt", "()I"),
        30
    );
}

#[test]
fn arraylist_remove_at_first() {
    assert_eq!(
        run_bootstrap_int("ArrayListExtTest.class", "testRemoveAtFirst", "()I"),
        1
    );
}

#[test]
fn arraylist_remove_obj() {
    assert_eq!(
        run_bootstrap_int("ArrayListExtTest.class", "testRemoveObj", "()I"),
        2
    );
}

#[test]
fn arraylist_contains_true() {
    assert_eq!(
        run_bootstrap_int("ArrayListExtTest.class", "testContainsTrue", "()I"),
        1
    );
}

#[test]
fn arraylist_contains_false() {
    assert_eq!(
        run_bootstrap_int("ArrayListExtTest.class", "testContainsFalse", "()I"),
        0
    );
}

#[test]
fn arraylist_clear() {
    assert_eq!(
        run_bootstrap_int("ArrayListExtTest.class", "testClear", "()I"),
        0
    );
}

#[test]
fn arraylist_is_empty() {
    assert_eq!(
        run_bootstrap_int("ArrayListExtTest.class", "testIsEmpty", "()I"),
        1
    );
}

#[test]
fn arraylist_set() {
    assert_eq!(
        run_bootstrap_int("ArrayListExtTest.class", "testSet", "()I"),
        99
    );
}

#[test]
fn arraylist_index_of() {
    assert_eq!(
        run_bootstrap_int("ArrayListExtTest.class", "testIndexOf", "()I"),
        1
    );
}

#[test]
fn arraylist_index_of_missing() {
    assert_eq!(
        run_bootstrap_int("ArrayListExtTest.class", "testIndexOfMissing", "()I"),
        -1
    );
}

#[test]
fn arraylist_add_at() {
    assert_eq!(
        run_bootstrap_int("ArrayListExtTest.class", "testAddAt", "()I"),
        2
    );
}

// ---- Phase 29: HashMap extended methods ----

#[test]
fn hashmap_put_if_absent_new() {
    assert_eq!(
        run_bootstrap_int("HashMapExtTest.class", "testPutIfAbsentNew", "()I"),
        42
    );
}

#[test]
fn hashmap_put_if_absent_existing() {
    assert_eq!(
        run_bootstrap_int("HashMapExtTest.class", "testPutIfAbsentExisting", "()I"),
        100
    );
}

#[test]
fn hashmap_put_if_absent_no_overwrite() {
    assert_eq!(
        run_bootstrap_int("HashMapExtTest.class", "testPutIfAbsentNoOverwrite", "()I"),
        7
    );
}

#[test]
fn hashmap_clear() {
    assert_eq!(
        run_bootstrap_int("HashMapExtTest.class", "testClear", "()I"),
        0
    );
}

#[test]
fn hashmap_contains_value_true() {
    assert_eq!(
        run_bootstrap_int("HashMapExtTest.class", "testContainsValueTrue", "()I"),
        1
    );
}

#[test]
fn hashmap_contains_value_false() {
    assert_eq!(
        run_bootstrap_int("HashMapExtTest.class", "testContainsValueFalse", "()I"),
        0
    );
}

// ---- Phase 29: System.err + utilities ----

#[test]
fn system_err_println() {
    assert_eq!(
        run_bootstrap_int("SystemExtTest.class", "testErr", "()I"),
        1
    );
}

#[test]
fn system_line_separator() {
    assert_eq!(
        run_bootstrap_int("SystemExtTest.class", "testLineSeparator", "()I"),
        1
    );
}

#[test]
fn system_identity_hash_code() {
    assert_eq!(
        run_bootstrap_int("SystemExtTest.class", "testIdentityHashCode", "()I"),
        1
    );
}

#[test]
fn system_identity_hash_code_null() {
    assert_eq!(
        run_bootstrap_int("SystemExtTest.class", "testIdentityHashCodeNull", "()I"),
        0
    );
}

#[test]
fn system_get_security_manager_returns_null() {
    assert_eq!(
        run_bootstrap_int("SystemExtTest.class", "testGetSecurityManager", "()I"),
        1
    );
}

// ---- Phase 30: Integer/Long bit ops, Objects, Collections utilities ----

#[test]
fn integer_bitcount() {
    assert_eq!(
        run_bootstrap_int("IntegerBitOpsTest.class", "testBitCount", "()I"),
        8
    );
}

#[test]
fn integer_bitcount_zero() {
    assert_eq!(
        run_bootstrap_int("IntegerBitOpsTest.class", "testBitCountZero", "()I"),
        0
    );
}

#[test]
fn integer_leading_zeros() {
    assert_eq!(
        run_bootstrap_int("IntegerBitOpsTest.class", "testLeadingZeros", "()I"),
        31
    );
}

#[test]
fn integer_trailing_zeros() {
    assert_eq!(
        run_bootstrap_int("IntegerBitOpsTest.class", "testTrailingZeros", "()I"),
        3
    );
}

#[test]
fn integer_highest_one_bit() {
    assert_eq!(
        run_bootstrap_int("IntegerBitOpsTest.class", "testHighestOneBit", "()I"),
        64
    );
}

#[test]
fn integer_lowest_one_bit() {
    assert_eq!(
        run_bootstrap_int("IntegerBitOpsTest.class", "testLowestOneBit", "()I"),
        4
    );
}

#[test]
fn integer_signum_positive() {
    assert_eq!(
        run_bootstrap_int("IntegerBitOpsTest.class", "testSignumPositive", "()I"),
        1
    );
}

#[test]
fn integer_signum_negative() {
    assert_eq!(
        run_bootstrap_int("IntegerBitOpsTest.class", "testSignumNegative", "()I"),
        -1
    );
}

#[test]
fn integer_signum_zero() {
    assert_eq!(
        run_bootstrap_int("IntegerBitOpsTest.class", "testSignumZero", "()I"),
        0
    );
}

#[test]
fn integer_compare_static() {
    assert_eq!(
        run_bootstrap_int("IntegerBitOpsTest.class", "testCompare", "()I"),
        1
    );
}

#[test]
fn integer_sum() {
    assert_eq!(
        run_bootstrap_int("IntegerBitOpsTest.class", "testSum", "()I"),
        20
    );
}

#[test]
fn integer_max_static() {
    assert_eq!(
        run_bootstrap_int("IntegerBitOpsTest.class", "testMax", "()I"),
        9
    );
}

#[test]
fn integer_min_static() {
    assert_eq!(
        run_bootstrap_int("IntegerBitOpsTest.class", "testMin", "()I"),
        3
    );
}

#[test]
fn long_bitcount() {
    assert_eq!(
        run_bootstrap_int("LongBitOpsTest.class", "testBitCount", "()I"),
        8
    );
}

#[test]
fn long_leading_zeros() {
    assert_eq!(
        run_bootstrap_int("LongBitOpsTest.class", "testLeadingZeros", "()I"),
        63
    );
}

#[test]
fn long_trailing_zeros() {
    assert_eq!(
        run_bootstrap_int("LongBitOpsTest.class", "testTrailingZeros", "()I"),
        3
    );
}

#[test]
fn long_highest_one_bit() {
    assert_eq!(
        run_bootstrap_int("LongBitOpsTest.class", "testHighestOneBit", "()I"),
        64
    );
}

#[test]
fn long_lowest_one_bit() {
    assert_eq!(
        run_bootstrap_int("LongBitOpsTest.class", "testLowestOneBit", "()I"),
        4
    );
}

#[test]
fn long_signum_positive() {
    assert_eq!(
        run_bootstrap_int("LongBitOpsTest.class", "testSignumPositive", "()I"),
        1
    );
}

#[test]
fn long_signum_negative() {
    assert_eq!(
        run_bootstrap_int("LongBitOpsTest.class", "testSignumNegative", "()I"),
        -1
    );
}

#[test]
fn long_signum_zero() {
    assert_eq!(
        run_bootstrap_int("LongBitOpsTest.class", "testSignumZero", "()I"),
        0
    );
}

#[test]
fn long_compare_static() {
    assert_eq!(
        run_bootstrap_int("LongBitOpsTest.class", "testCompare", "()I"),
        1
    );
}

#[test]
fn long_sum() {
    assert_eq!(
        run_bootstrap_int("LongBitOpsTest.class", "testSum", "()I"),
        20
    );
}

#[test]
fn objects_is_null_true() {
    assert_eq!(
        run_bootstrap_int("ObjectsTest.class", "testIsNull", "()I"),
        1
    );
}

#[test]
fn objects_is_null_false() {
    assert_eq!(
        run_bootstrap_int("ObjectsTest.class", "testIsNullFalse", "()I"),
        0
    );
}

#[test]
fn objects_non_null_true() {
    assert_eq!(
        run_bootstrap_int("ObjectsTest.class", "testNonNull", "()I"),
        1
    );
}

#[test]
fn objects_non_null_false() {
    assert_eq!(
        run_bootstrap_int("ObjectsTest.class", "testNonNullFalse", "()I"),
        0
    );
}

#[test]
fn objects_require_non_null() {
    assert_eq!(
        run_bootstrap_int("ObjectsTest.class", "testRequireNonNull", "()I"),
        2
    );
}

#[test]
fn objects_equals_same() {
    assert_eq!(
        run_bootstrap_int("ObjectsTest.class", "testEquals", "()I"),
        1
    );
}

#[test]
fn objects_equals_both_null() {
    assert_eq!(
        run_bootstrap_int("ObjectsTest.class", "testEqualsBothNull", "()I"),
        1
    );
}

#[test]
fn objects_equals_one_null() {
    assert_eq!(
        run_bootstrap_int("ObjectsTest.class", "testEqualsOneNull", "()I"),
        0
    );
}

#[test]
fn objects_hash_code_null() {
    assert_eq!(
        run_bootstrap_int("ObjectsTest.class", "testHashCode", "()I"),
        0
    );
}

#[test]
fn collections_empty_list_size() {
    assert_eq!(
        run_bootstrap_int("CollectionsUtilTest.class", "testEmptyList", "()I"),
        0
    );
}

#[test]
fn collections_singleton_list_size() {
    assert_eq!(
        run_bootstrap_int("CollectionsUtilTest.class", "testSingletonList", "()I"),
        1
    );
}

#[test]
fn collections_singleton_list_get() {
    assert_eq!(
        run_bootstrap_int("CollectionsUtilTest.class", "testSingletonListGet", "()I"),
        99
    );
}

#[test]
fn collections_reverse_first_element() {
    assert_eq!(
        run_bootstrap_int("CollectionsUtilTest.class", "testReverse", "()I"),
        3
    );
}

#[test]
fn collections_reverse_size_unchanged() {
    assert_eq!(
        run_bootstrap_int("CollectionsUtilTest.class", "testReverseSize", "()I"),
        2
    );
}

#[test]
fn collections_frequency() {
    assert_eq!(
        run_bootstrap_int("CollectionsUtilTest.class", "testFrequency", "()I"),
        2
    );
}

// ---- Phase 31: String extensions, StringBuilder extensions, Arrays.asList ----

#[test]
fn string_strip() {
    assert_eq!(
        run_bootstrap_int("StringExtTest.class", "testStrip", "()I"),
        5
    );
}

#[test]
fn string_strip_leading() {
    assert_eq!(
        run_bootstrap_int("StringExtTest.class", "testStripLeading", "()I"),
        2
    );
}

#[test]
fn string_strip_trailing() {
    assert_eq!(
        run_bootstrap_int("StringExtTest.class", "testStripTrailing", "()I"),
        2
    );
}

#[test]
fn string_is_blank_true() {
    assert_eq!(
        run_bootstrap_int("StringExtTest.class", "testIsBlankTrue", "()I"),
        1
    );
}

#[test]
fn string_is_blank_false() {
    assert_eq!(
        run_bootstrap_int("StringExtTest.class", "testIsBlankFalse", "()I"),
        0
    );
}

#[test]
fn string_repeat() {
    assert_eq!(
        run_bootstrap_int("StringExtTest.class", "testRepeat", "()I"),
        6
    );
}

#[test]
fn string_repeat_zero() {
    assert_eq!(
        run_bootstrap_int("StringExtTest.class", "testRepeatZero", "()I"),
        0
    );
}

#[test]
fn string_join() {
    assert_eq!(
        run_bootstrap_int("StringExtTest.class", "testJoin", "()I"),
        5
    );
}

#[test]
fn string_index_of_char() {
    assert_eq!(
        run_bootstrap_int("StringExtTest.class", "testIndexOfChar", "()I"),
        2
    );
}

#[test]
fn string_index_of_char_missing() {
    assert_eq!(
        run_bootstrap_int("StringExtTest.class", "testIndexOfCharMissing", "()I"),
        -1
    );
}

#[test]
fn string_last_index_of() {
    assert_eq!(
        run_bootstrap_int("StringExtTest.class", "testLastIndexOf", "()I"),
        4
    );
}

#[test]
fn sb_insert_string_length() {
    assert_eq!(
        run_bootstrap_int("StringBuilderExtTest.class", "testInsertString", "()I"),
        7
    );
}

#[test]
fn sb_insert_string_value() {
    assert_eq!(
        run_bootstrap_int("StringBuilderExtTest.class", "testInsertStringValue", "()I"),
        1
    );
}

#[test]
fn sb_delete() {
    assert_eq!(
        run_bootstrap_int("StringBuilderExtTest.class", "testDelete", "()I"),
        3
    );
}

#[test]
fn sb_delete_char_at() {
    assert_eq!(
        run_bootstrap_int("StringBuilderExtTest.class", "testDeleteCharAt", "()I"),
        4
    );
}

#[test]
fn sb_reverse() {
    assert_eq!(
        run_bootstrap_int("StringBuilderExtTest.class", "testReverse", "()I"),
        1
    );
}

#[test]
fn sb_char_at() {
    assert_eq!(
        run_bootstrap_int("StringBuilderExtTest.class", "testCharAt", "()I"),
        101 // 'e'
    );
}

#[test]
fn sb_set_length() {
    assert_eq!(
        run_bootstrap_int("StringBuilderExtTest.class", "testSetLength", "()I"),
        3
    );
}

#[test]
fn arrays_as_list_size() {
    assert_eq!(
        run_bootstrap_int("ArraysAsListTest.class", "testSize", "()I"),
        3
    );
}

#[test]
fn arrays_as_list_get() {
    assert_eq!(
        run_bootstrap_int("ArraysAsListTest.class", "testGet", "()I"),
        1
    );
}

#[test]
fn arrays_as_list_empty() {
    assert_eq!(
        run_bootstrap_int("ArraysAsListTest.class", "testEmptyArray", "()I"),
        0
    );
}

// ---- Phase 32: Throwable.getMessage, List/Set/Map.of, Optional, bulk ops ----

#[test]
fn throwable_get_message() {
    assert_eq!(
        run_bootstrap_int("ThrowableTest.class", "testGetMessage", "()I"),
        5
    );
}

#[test]
fn throwable_get_message_null() {
    assert_eq!(
        run_bootstrap_int("ThrowableTest.class", "testGetMessageNull", "()I"),
        1
    );
}

#[test]
fn throwable_tostring() {
    assert_eq!(
        run_bootstrap_int("ThrowableTest.class", "testToString", "()I"),
        1
    );
}

#[test]
fn throwable_catch_get_message() {
    assert_eq!(
        run_bootstrap_int("ThrowableTest.class", "testCatchGetMessage", "()I"),
        4
    );
}

#[test]
fn throwable_stack_trace_basic_frames() {
    let frames = run_bootstrap_int("StackTraceBasicTest.class", "testBasicFrames", "()I");
    assert!(
        frames >= 3,
        "expected at least three real frames, got fixture code {frames}"
    );
}

#[test]
fn throwable_stack_trace_defensive_copy_and_setter() {
    assert_eq!(
        run_bootstrap_int(
            "StackTraceBasicTest.class",
            "testDefensiveCopyAndSetStackTrace",
            "()I"
        ),
        1
    );
}

#[test]
fn throwable_localized_message_still_falls_back_to_message() {
    assert_eq!(
        run_bootstrap_int(
            "StackTraceBasicTest.class",
            "testLocalizedMessageStillFallsBack",
            "()I"
        ),
        1
    );
}

#[test]
fn throwable_print_stack_trace_includes_cause_chain() {
    assert_eq!(
        run_bootstrap_int(
            "StackTraceCausedByTest.class",
            "testPrintStackTraceCause",
            "()I"
        ),
        1
    );
}

#[test]
fn throwable_suppressed_exceptions_are_retained() {
    assert_eq!(
        run_bootstrap_int(
            "StackTraceSuppressedTest.class",
            "testSuppressedIsRetained",
            "()I"
        ),
        1
    );
}

#[test]
fn throwable_lambda_trace_keeps_user_frame() {
    assert_eq!(
        run_bootstrap_int(
            "StackTraceLambdaTest.class",
            "testLambdaTraceIncludesUserCaller",
            "()I"
        ),
        1
    );
}

#[test]
fn throwable_print_stack_trace_format_matches_golden_shape() {
    assert_eq!(
        run_bootstrap_int(
            "StackTraceFormatGoldenTest.class",
            "testPrintStackTraceFormat",
            "()I"
        ),
        1
    );
}

#[test]
fn stack_trace_line_lookup_uses_deepest_preceding_bci() {
    let table = vec![(0, 10), (4, 12), (12, 30)];
    assert_eq!(line_number_for_bci(&table, 0), 10);
    assert_eq!(line_number_for_bci(&table, 7), 12);
    assert_eq!(line_number_for_bci(&table, 99), 30);
}

#[test]
fn stack_trace_line_lookup_returns_unknown_for_empty_or_before_first_entry() {
    assert_eq!(line_number_for_bci(&[], 7), -1);
    assert_eq!(line_number_for_bci(&[(10, 40)], 7), -1);
}

#[test]
fn list_of_zero() {
    assert_eq!(
        run_bootstrap_int("ListOfTest.class", "testListOfZero", "()I"),
        0
    );
}

#[test]
fn list_of_one() {
    assert_eq!(
        run_bootstrap_int("ListOfTest.class", "testListOfOne", "()I"),
        1
    );
}

#[test]
fn list_of_three() {
    assert_eq!(
        run_bootstrap_int("ListOfTest.class", "testListOfThree", "()I"),
        3
    );
}

#[test]
fn list_of_get() {
    assert_eq!(
        run_bootstrap_int("ListOfTest.class", "testListOfGet", "()I"),
        1
    );
}

#[test]
fn set_of_two() {
    assert_eq!(
        run_bootstrap_int("ListOfTest.class", "testSetOfTwo", "()I"),
        2
    );
}

#[test]
fn map_of_one() {
    assert_eq!(
        run_bootstrap_int("ListOfTest.class", "testMapOfOne", "()I"),
        5
    );
}

#[test]
fn map_of_two() {
    assert_eq!(
        run_bootstrap_int("ListOfTest.class", "testMapOfTwo", "()I"),
        2
    );
}

#[test]
fn optional_empty() {
    assert_eq!(
        run_bootstrap_int("OptionalTest.class", "testEmpty", "()I"),
        0
    );
}

#[test]
fn optional_of() {
    assert_eq!(run_bootstrap_int("OptionalTest.class", "testOf", "()I"), 1);
}

#[test]
fn optional_get() {
    assert_eq!(run_bootstrap_int("OptionalTest.class", "testGet", "()I"), 5);
}

#[test]
fn optional_or_else_empty() {
    assert_eq!(
        run_bootstrap_int("OptionalTest.class", "testOrElse", "()I"),
        7
    );
}

#[test]
fn optional_or_else_present() {
    assert_eq!(
        run_bootstrap_int("OptionalTest.class", "testOrElsePresent", "()I"),
        2
    );
}

#[test]
fn optional_is_empty() {
    assert_eq!(
        run_bootstrap_int("OptionalTest.class", "testIsEmpty", "()I"),
        1
    );
}

#[test]
fn optional_of_nullable_null() {
    assert_eq!(
        run_bootstrap_int("OptionalTest.class", "testOfNullable", "()I"),
        0
    );
}

#[test]
fn arraylist_add_all() {
    assert_eq!(
        run_bootstrap_int("CollectionBulkTest.class", "testAddAll", "()I"),
        4
    );
}

#[test]
fn arraylist_add_all_empty_returns_false() {
    assert_eq!(
        run_bootstrap_int("CollectionBulkTest.class", "testAddAllEmpty", "()I"),
        0
    );
}

#[test]
fn hashmap_put_all() {
    assert_eq!(
        run_bootstrap_int("CollectionBulkTest.class", "testPutAll", "()I"),
        3
    );
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
fn nested_clinit_gc_preserves_caller_locals() {
    // Regression: a GC fired inside a NESTED `run_execution` (a class `<clinit>`
    // re-entered from the caller) must treat the suspended caller's frame as a
    // GC root. Otherwise the caller's live locals are reclaimed while still young
    // and their young-gen indices reused, silently corrupting the caller's state.
    //
    // `NestedClinitGcOuter.run` keeps the ONLY reference to an int[]{1234567} in
    // a local, then reads `NestedClinitGcInner.VALUE`, triggering that class's
    // `<clinit>`. That `<clinit>` allocates 1000 short-lived arrays, forcing a
    // collection while `run`'s frame is suspended on the Rust stack, then keeps
    // allocating (reusing the freed young-gen indices). With the frame correctly
    // rooted, `guarded[0]` (1234567) plus `VALUE` (0+..+999 = 499500) is 1734067.
    // Before the fix the guarded array was collected and its young slot reused,
    // yielding a wrong value (or a type error).
    let outer = load_class_context("NestedClinitGcOuter.class");
    let inner = load_class_context("NestedClinitGcInner.class");
    let entry_class = outer.class_name.clone();
    let mut registry = ClassRegistry::new();
    registry.register(outer);
    registry.register(inner);
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
        "run",
        "()I",
        &[],
    );
    assert_eq!(
        result.expect("nested-clinit GC run must not error"),
        Some(Slot::Int(1_734_067)),
        "caller's live local was corrupted by a GC during the nested <clinit>"
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
        sam_interface: "java/util/function/IntSupplier".to_string(),
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
            _ => Err(Error::TypeMismatch {
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
        sam_interface: "java/util/function/IntUnaryOperator".to_string(),
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
                _ => return Err(Error::NullPointerException),
            };
            let n: i32 = s.parse().map_err(|_| Error::NullPointerException)?;
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
                return Err(Error::NullPointerException);
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
        load_source: ClassLoadSource::Classfile,
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
                return Err(Error::NullPointerException);
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
    let r1_first = make_int(&mut heap, 1);
    let r1_second = make_int(&mut heap, 1);
    let r2 = make_int(&mut heap, 2);
    heap.get_mut(list).unwrap().fields[1] = Slot::Reference(Some(r3));
    heap.get_mut(list).unwrap().fields[2] = Slot::Reference(Some(r1_first));
    heap.get_mut(list).unwrap().fields[3] = Slot::Reference(Some(r1_second));
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
        matches!(result, Err(Error::NullPointerException)),
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

// ---- Issue 649: java.util.Properties ----

#[test]
fn properties_loads_sample_file_with_escapes_and_continuations() {
    let sample_path = fixture("sample.properties");

    let result = run_bootstrap_with_string_args(
        "PropertiesBasicTest.class",
        "loadSample",
        "(Ljava/lang/String;)I",
        &[sample_path.to_string_lossy().into_owned()],
    );

    assert_eq!(result.unwrap(), Some(Slot::Int(0)));
}

#[test]
fn properties_get_property_overloads_handle_defaults() {
    assert_eq!(
        run_bootstrap_int("PropertiesBasicTest.class", "getPropertyDefaults", "()I"),
        0
    );
}

#[test]
fn properties_chained_defaults_are_consulted_after_local_map() {
    assert_eq!(
        run_bootstrap_int("PropertiesBasicTest.class", "chainedDefaults", "()I"),
        0
    );
}

#[test]
fn properties_set_property_returns_prior_value() {
    assert_eq!(
        run_bootstrap_int(
            "PropertiesBasicTest.class",
            "setPropertyReturnsPriorValue",
            "()I"
        ),
        0
    );
}

#[test]
fn properties_store_round_trips_loaded_map() {
    let (root, _cleanup) = make_temp_root("duke-properties-store");
    let output_path = root.join("roundtrip.properties");

    let result = run_bootstrap_with_string_args(
        "PropertiesBasicTest.class",
        "storeRoundTrip",
        "(Ljava/lang/String;)I",
        &[output_path.to_string_lossy().into_owned()],
    );

    assert_eq!(result.unwrap(), Some(Slot::Int(0)));
}

#[test]
fn properties_store_escapes_special_keys_and_values() {
    let (root, _cleanup) = make_temp_root("duke-properties-escaped-store");
    let output_path = root.join("escaped.properties");

    let result = run_bootstrap_with_string_args(
        "PropertiesBasicTest.class",
        "storeEscapedRoundTrip",
        "(Ljava/lang/String;)I",
        &[output_path.to_string_lossy().into_owned()],
    );

    assert_eq!(result.unwrap(), Some(Slot::Int(0)));
}

#[test]
fn properties_property_name_views_include_defaults() {
    assert_eq!(
        run_bootstrap_int(
            "PropertiesBasicTest.class",
            "propertyNamesIncludeDefaults",
            "()I"
        ),
        0
    );
}

#[test]
fn properties_local_views_and_mutation_ignore_defaults() {
    assert_eq!(
        run_bootstrap_int("PropertiesBasicTest.class", "localViewsAndMutation", "()I"),
        0
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
                Err(Error::Unimplemented {
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
        Err(Error::Unimplemented {
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
            Err(Error::Unimplemented {
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
        Err(Error::Unimplemented {
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
        sam_interface: "java/lang/Runnable".to_string(),
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
    ) -> Result<Option<Slot>> {
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
        load_source: ClassLoadSource::Classfile,
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
        load_source: ClassLoadSource::Classfile,
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
    native_println_boolean(&[Slot::Reference(None), Slot::Int(0)], &mut heap, &mut out).unwrap();
    assert_eq!(String::from_utf8(out).unwrap().trim(), "false");
}

#[test]
fn native_println_boolean_true() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    native_println_boolean(&[Slot::Reference(None), Slot::Int(1)], &mut heap, &mut out).unwrap();
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
    assert!(matches!(err, Error::TypeMismatch { .. }));
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
    assert!(matches!(err, Error::TypeMismatch { .. }));
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
// native_string_equalsignorecase (issue #854)
// ---------------------------------------------------------------------------

#[test]
fn native_string_equalsignorecase_covers_contract() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();

    // Helper: allocate two strings and run the native, returning the bool result.
    let mut run = |heap: &mut duke_gc::Heap, a: &str, b: Option<&str>| -> i32 {
        let this = heap.allocate_string(a.to_string());
        let other = b.map_or(Slot::Reference(None), |s| {
            Slot::Reference(Some(heap.allocate_string(s.to_string())))
        });
        match native_string_equalsignorecase(&[Slot::Reference(Some(this)), other], heap, &mut out)
            .unwrap()
            .unwrap()
        {
            Slot::Int(v) => v,
            other => panic!("expected Int, got {other:?}"),
        }
    };

    // Null arg -> false.
    assert_eq!(run(&mut heap, "INFO", None), 0);
    // Length mismatch -> false.
    assert_eq!(run(&mut heap, "abc", Some("ab")), 0);
    // ASCII case fold.
    assert_eq!(run(&mut heap, "INFO", Some("info")), 1);
    assert_eq!(run(&mut heap, "Info", Some("iNfO")), 1);
    assert_eq!(run(&mut heap, "info", Some("warn")), 0);
    // Two-step locale-independent fold: KELVIN SIGN (U+212A) folds to 'k'.
    assert_eq!(run(&mut heap, "k", Some("\u{212A}")), 1);
    // Reflexive on identical content.
    assert_eq!(run(&mut heap, "hello", Some("hello")), 1);
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
        matches!(err, Error::SystemExit { code: 42 }),
        "expected SystemExit(42), got {err:?}"
    );
}

#[test]
fn native_system_exit_non_int_arg_uses_code_1() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let err = native_system_exit(&[], &mut heap, &mut out).unwrap_err();
    assert!(
        matches!(err, Error::SystemExit { code: 1 }),
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
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// format_arg
// ---------------------------------------------------------------------------

#[test]
fn format_arg_long_d_spec() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Long(12345_i64);
    let result = format_arg('d', "", None, None, &Slot::Reference(Some(r)), &heap).unwrap();
    assert_eq!(result, "12345");
}

#[test]
fn format_arg_double_f_spec() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Double".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Double(std::f64::consts::PI);
    let result = format_arg('f', "", None, Some(2), &Slot::Reference(Some(r)), &heap).unwrap();
    assert_eq!(result, "3.14");
}

#[test]
fn format_arg_float_f_spec() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Float".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Float(1.5_f32);
    let result = format_arg('f', "", None, Some(1), &Slot::Reference(Some(r)), &heap).unwrap();
    assert_eq!(result, "1.5");
}

#[test]
fn format_arg_int_x_spec() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(255);
    let result = format_arg('x', "", None, None, &Slot::Reference(Some(r)), &heap).unwrap();
    assert_eq!(result, "ff");
}

#[test]
fn format_arg_int_uppercase_x_spec() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(255);
    let result = format_arg('X', "", None, None, &Slot::Reference(Some(r)), &heap).unwrap();
    assert_eq!(result, "FF");
}

#[test]
fn format_arg_long_x_spec() {
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Long(255_i64);
    let result = format_arg('x', "", None, None, &Slot::Reference(Some(r)), &heap).unwrap();
    assert_eq!(result, "ff");
}

#[test]
fn format_arg_null_returns_null_string() {
    let heap = duke_gc::Heap::new();
    let result = format_arg('s', "", None, None, &Slot::Reference(None), &heap).unwrap();
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
    let slot =
        execute_string_concat_recipe("\u{1}", &[Slot::Int(42)], &['I'], &[], &mut heap).unwrap();
    let r = slot.as_reference().unwrap();
    assert_eq!(heap.get(r).unwrap().string_value.as_deref(), Some("42"));
}

#[test]
fn execute_string_concat_recipe_constant_only() {
    let mut heap = duke_gc::Heap::new();
    let slot =
        execute_string_concat_recipe("\u{2}", &[], &[], &["hello".to_string()], &mut heap).unwrap();
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
    let slot =
        execute_string_concat_recipe("x\u{1}y", &[Slot::Int(5)], &['I'], &[], &mut heap).unwrap();
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
    use duke_classfile::{CpEntry, CpIndex};
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
    use duke_classfile::{CpEntry, CpIndex};
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
    assert!(matches!(err, Error::NullPointerException));
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
    assert!(matches!(err, Error::NullPointerException));
}

// ---------------------------------------------------------------------------
// null-arm: Integer.parseInt, Long.parseLong, Double.parseDouble,
//           Float.parseFloat, Boolean.parseBoolean
// ---------------------------------------------------------------------------

#[test]
fn native_integer_parseint_null_raises_npe() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let err = native_integer_parseint(&[Slot::Reference(None)], &mut heap, &mut out).unwrap_err();
    assert!(matches!(err, Error::NullPointerException));
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
    assert!(matches!(err, Error::NullPointerException));
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
    let err = native_double_parsedouble(&[Slot::Reference(None)], &mut heap, &mut out).unwrap_err();
    assert!(matches!(err, Error::NullPointerException));
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
    let err = native_float_parsefloat(&[Slot::Reference(None)], &mut heap, &mut out).unwrap_err();
    assert!(matches!(err, Error::NullPointerException));
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
    assert!(matches!(err, Error::NullPointerException));
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
    assert!(matches!(err, Error::NullPointerException));
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
    assert!(matches!(err, Error::NullPointerException));
}

// ---------------------------------------------------------------------------
// native_math_min_double
// ---------------------------------------------------------------------------

#[test]
fn native_math_min_double_returns_smaller() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let r = native_math_min_double(&[Slot::Double(3.0), Slot::Double(1.5)], &mut heap, &mut out)
        .unwrap()
        .unwrap();
    assert!(matches!(r, Slot::Double(v) if (v - 1.5).abs() < 1e-9));
}

#[test]
fn native_math_min_double_returns_first_when_equal() {
    let mut heap = duke_gc::Heap::new();
    let mut out: Vec<u8> = Vec::new();
    let r = native_math_min_double(&[Slot::Double(2.0), Slot::Double(2.0)], &mut heap, &mut out)
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
    use duke_bytecode::ArrayType;
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
    use duke_bytecode::ArrayType;
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
    use duke_bytecode::ArrayType;
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
    use duke_bytecode::ArrayType;
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
    use duke_bytecode::ArrayType;
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
}

#[test]
fn execute_fastore_and_faload_roundtrip() {
    use duke_bytecode::ArrayType;
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
    use duke_bytecode::ArrayType;
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
    use duke_bytecode::ArrayType;
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
}

#[test]
fn execute_daload_out_of_bounds_raises_error() {
    use duke_bytecode::ArrayType;
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
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
    use duke_classfile::CpIndex;
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
    use duke_classfile::CpIndex;
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
    use duke_classfile::CpIndex;
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
        load_source: ClassLoadSource::Classfile,
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
        load_source: ClassLoadSource::Classfile,
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
        load_source: ClassLoadSource::Classfile,
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
        load_source: ClassLoadSource::Classfile,
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
    assert!(matches!(err, Error::NegativeArraySize { size: -1 }));
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
    assert!(matches!(err, Error::NegativeArraySize { size: -2 }));
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
) -> Result<Option<Slot>> {
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
        is_native: false,
        is_abstract: false,
        instructions: instructions.into(),
        max_stack,
        max_locals,
        exception_table: vec![],
        pc_to_idx: Arc::new(pc_to_idx),
        line_number_table: Vec::new(),
        source_file: None,
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
        load_source: ClassLoadSource::Classfile,
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
        Error::NullPointerException
    ));
}

#[test]
fn test_extract_ref_arg_type_mismatch() {
    let args = vec![Slot::Int(1)];
    assert!(matches!(
        extract_ref_arg(&args, 0).unwrap_err(),
        Error::NullPointerException
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
        Error::TypeMismatch {
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
        Error::TypeMismatch {
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
        Error::TypeMismatch {
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
        Error::TypeMismatch {
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
        Error::TypeMismatch {
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
        Error::TypeMismatch {
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
        Error::TypeMismatch {
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
        Error::JavaException { .. }
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
        Error::JavaException { .. }
    ));
}

// ---- execute_class: Newarray init for Long/Float/Double ----

#[test]
fn ec_newarray_long_default_slot_is_long_zero() {
    // Create long[1]; arm deleted → fields stay Int(0) → Laload TypeMismatch
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Iconst1),
            (1, Instruction::Newarray(duke_bytecode::ArrayType::Long)),
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
            (1, Instruction::Newarray(duke_bytecode::ArrayType::Float)),
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
            (1, Instruction::Newarray(duke_bytecode::ArrayType::Double)),
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
            (2, Instruction::Newarray(duke_bytecode::ArrayType::Int)),
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
            (2, Instruction::Newarray(duke_bytecode::ArrayType::Int)),
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
            (2, Instruction::Newarray(duke_bytecode::ArrayType::Int)),
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
    assert!(
        matches!(&err, Error::JavaException { class_name } if class_name == "java/lang/ArrayIndexOutOfBoundsException"),
        "expected ArrayIndexOutOfBoundsException, got {err:?}"
    );
}

/// Iastore OOB at length; kills its || → && mutant.
#[test]
fn ec_iastore_oob_at_length_errors() {
    let err = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (2, Instruction::Newarray(duke_bytecode::ArrayType::Int)),
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
    assert!(
        matches!(&err, Error::JavaException { class_name } if class_name == "java/lang/ArrayIndexOutOfBoundsException"),
        "expected ArrayIndexOutOfBoundsException, got {err:?}"
    );
}

/// Laload: valid idx=0 succeeds (kills < → == at Laload bounds).
#[test]
fn ec_laload_valid_idx0_succeeds() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (2, Instruction::Newarray(duke_bytecode::ArrayType::Long)),
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
            (2, Instruction::Newarray(duke_bytecode::ArrayType::Long)),
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
            (2, Instruction::Newarray(duke_bytecode::ArrayType::Long)),
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
    assert!(
        matches!(&err, Error::JavaException { class_name } if class_name == "java/lang/ArrayIndexOutOfBoundsException"),
        "expected ArrayIndexOutOfBoundsException, got {err:?}"
    );
}

/// Lastore OOB kills || → &&.
#[test]
fn ec_lastore_oob_at_length_errors() {
    let err = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (2, Instruction::Newarray(duke_bytecode::ArrayType::Long)),
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
    assert!(
        matches!(&err, Error::JavaException { class_name } if class_name == "java/lang/ArrayIndexOutOfBoundsException"),
        "expected ArrayIndexOutOfBoundsException, got {err:?}"
    );
}

/// Faload valid idx=0 and idx=1.
#[test]
fn ec_faload_valid_idx0_succeeds() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (2, Instruction::Newarray(duke_bytecode::ArrayType::Float)),
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
            (2, Instruction::Newarray(duke_bytecode::ArrayType::Float)),
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
    assert!(
        matches!(&err, Error::JavaException { class_name } if class_name == "java/lang/ArrayIndexOutOfBoundsException"),
        "expected ArrayIndexOutOfBoundsException, got {err:?}"
    );
}

#[test]
fn ec_fastore_oob_at_length_errors() {
    let err = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (2, Instruction::Newarray(duke_bytecode::ArrayType::Float)),
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
    assert!(
        matches!(&err, Error::JavaException { class_name } if class_name == "java/lang/ArrayIndexOutOfBoundsException"),
        "expected ArrayIndexOutOfBoundsException, got {err:?}"
    );
}

#[test]
fn ec_daload_valid_idx0_succeeds() {
    let r = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (2, Instruction::Newarray(duke_bytecode::ArrayType::Double)),
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
            (2, Instruction::Newarray(duke_bytecode::ArrayType::Double)),
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
            (2, Instruction::Newarray(duke_bytecode::ArrayType::Double)),
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
    assert!(
        matches!(&err, Error::JavaException { class_name } if class_name == "java/lang/ArrayIndexOutOfBoundsException"),
        "expected ArrayIndexOutOfBoundsException, got {err:?}"
    );
}

#[test]
fn ec_dastore_oob_at_length_errors() {
    let err = execute_class_synthetic(
        vec![
            (0, Instruction::Bipush(3)),
            (2, Instruction::Newarray(duke_bytecode::ArrayType::Double)),
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
    assert!(
        matches!(&err, Error::JavaException { class_name } if class_name == "java/lang/ArrayIndexOutOfBoundsException"),
        "expected ArrayIndexOutOfBoundsException, got {err:?}"
    );
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
    assert!(matches!(err, Error::DivisionByZero));
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
    assert!(matches!(err, Error::DivisionByZero));
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
    assert!(matches!(err, Error::NegativeArraySize { .. }));
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
        Error::ArrayIndexOutOfBounds {
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
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
    use duke_classfile::CpIndex;
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
    use duke_classfile::CpIndex;
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
    use duke_classfile::CpIndex;
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
    use duke_classfile::CpIndex;
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
}

// ---- format_arg: Long with %X (uppercase) ----

#[test]
fn format_arg_long_uppercase_x_spec() {
    // Mutant: delete Long arm → Long falls to _ → returns "0" instead of "FF"
    let mut heap = duke_gc::Heap::new();
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Long(255_i64);
    let result = format_arg('X', "", None, None, &Slot::Reference(Some(r)), &heap).unwrap();
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
    use duke_classfile::CpIndex;
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
    use duke_classfile::CpIndex;
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
    use duke_classfile::CpIndex;
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
    assert!(matches!(err, Error::NegativeArraySize { .. }));
}

// ---- execute(): Aaload bounds (line 4663: || → &&, < → ==, < → >, < → <=) ----

#[test]
fn execute_aaload_valid_idx0_returns_null() {
    // idx=0: kills < → == (0==0→error) and < → <= (0<=0→error)
    use duke_classfile::CpIndex;
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
    use duke_classfile::CpIndex;
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
    use duke_classfile::CpIndex;
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
}

// ---- execute(): Aastore bounds (line 4680: || → &&, < → ==, < → >, < → <=) ----

#[test]
fn execute_aastore_valid_idx0_stores() {
    // idx=0: kills < → == and < → <=; stack: ref, idx=0, null
    use duke_classfile::CpIndex;
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
    use duke_classfile::CpIndex;
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
    use duke_classfile::CpIndex;
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
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
    assert!(
        matches!(err, Error::ArrayIndexOutOfBounds { .. }),
        "expected ArrayIndexOutOfBounds, got {err:?}"
    );
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
    assert!(
        matches!(&err, Error::JavaException { class_name } if class_name == "java/lang/ArrayIndexOutOfBoundsException"),
        "expected ArrayIndexOutOfBoundsException, got {err:?}"
    );
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
    assert!(
        matches!(&err, Error::JavaException { class_name } if class_name == "java/lang/ArrayIndexOutOfBoundsException"),
        "expected ArrayIndexOutOfBoundsException, got {err:?}"
    );
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
    assert!(
        matches!(&err, Error::JavaException { class_name } if class_name == "java/lang/ArrayIndexOutOfBoundsException"),
        "expected ArrayIndexOutOfBoundsException, got {err:?}"
    );
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
    assert!(
        matches!(&err, Error::JavaException { class_name } if class_name == "java/lang/ArrayIndexOutOfBoundsException"),
        "expected ArrayIndexOutOfBoundsException, got {err:?}"
    );
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
    assert!(
        matches!(&err, Error::JavaException { class_name } if class_name == "java/lang/ArrayIndexOutOfBoundsException"),
        "expected ArrayIndexOutOfBoundsException, got {err:?}"
    );
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
    assert!(
        matches!(&err, Error::JavaException { class_name } if class_name == "java/lang/ArrayIndexOutOfBoundsException"),
        "expected ArrayIndexOutOfBoundsException, got {err:?}"
    );
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
    assert!(
        matches!(&err, Error::JavaException { class_name } if class_name == "java/lang/ArrayIndexOutOfBoundsException"),
        "expected ArrayIndexOutOfBoundsException, got {err:?}"
    );
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
    assert!(
        matches!(&err, Error::JavaException { class_name } if class_name == "java/lang/ArrayIndexOutOfBoundsException"),
        "expected ArrayIndexOutOfBoundsException, got {err:?}"
    );
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
    use duke_classfile::CpIndex;
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
    use duke_classfile::CpIndex;
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
        is_native: false,
        is_abstract: false,
        instructions: instructions.into(),
        max_stack: 4,
        max_locals: 1,
        exception_table: vec![],
        pc_to_idx: Arc::new(pc_to_idx),
        line_number_table: Vec::new(),
        source_file: None,
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
        load_source: ClassLoadSource::Classfile,
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
    assert!(matches!(err, Error::NegativeArraySize { .. }));
}

#[test]
fn ec_multianewarray_zero_dim_succeeds() {
    // dim=0: kills < → <= (0<=0=true→error; correct: 0<0=false→ok)
    use duke_classfile::CpIndex;
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
        is_native: false,
        is_abstract: false,
        instructions: instructions.into(),
        max_stack: 4,
        max_locals: 1,
        exception_table: vec![],
        pc_to_idx: Arc::new(pc_to_idx),
        line_number_table: Vec::new(),
        source_file: None,
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
        load_source: ClassLoadSource::Classfile,
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
    let r = execute_string_concat_recipe("\u{1}\u{1}", &[Slot::Int(42)], &['I'], &[], &mut heap)
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
    let r = execute_string_concat_recipe("\u{2}\u{2}", &[], &[], &["hello".to_string()], &mut heap)
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
    use duke_classfile::CpIndex;
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
        is_native: false,
        is_abstract: false,
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
        line_number_table: Vec::new(),
        source_file: None,
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
        load_source: ClassLoadSource::Classfile,
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
    use duke_classfile::CpIndex;
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
        is_native: false,
        is_abstract: false,
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
        line_number_table: Vec::new(),
        source_file: None,
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
        load_source: ClassLoadSource::Classfile,
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
    use duke_classfile::CpIndex;
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
        is_native: false,
        is_abstract: false,
        instructions: instructions.into(),
        max_stack: 4,
        max_locals: 0,
        exception_table: vec![],
        pc_to_idx: Arc::new(pc_to_idx),
        line_number_table: Vec::new(),
        source_file: None,
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
        load_source: ClassLoadSource::Classfile,
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
        load_source: ClassLoadSource::Classfile,
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
        matches!(err, Error::NegativeArraySize { .. }),
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

// Unit coverage for the caller-sensitive access rule that `Method.invoke` /
// `Constructor.newInstance` apply (see `caller_is_same_class` in native/reflect.rs):
// a class may reflectively access its OWN non-public members (same-class), while a
// different caller class still walls out. Both cases normalize away any
// `\0loader:N` key qualifier.
#[test]
fn caller_is_same_class_allows_same_class_and_rejects_cross_class() {
    fn control_with_caller(caller_class: &str) -> NativeControl {
        let mut control = NativeControl::default();
        control.set_stack_trace(vec![NativeStackFrame {
            class_name: caller_class.to_string(),
            method_name: "main".to_string(),
            file_name: None,
            line_number: -1,
        }]);
        control
    }

    // Same class → allowed (plain key, and loader-qualified caller vs plain
    // declaring key must still match after normalization).
    assert!(caller_is_same_class(
        &control_with_caller("com/example/Ladder"),
        "com/example/Ladder"
    ));
    assert!(caller_is_same_class(
        &control_with_caller("com/example/Ladder\0loader:7"),
        "com/example/Ladder"
    ));

    // Different caller class → rejected (this is the gson / ReflectionTest
    // cross-class case that must keep throwing IllegalAccessException).
    assert!(!caller_is_same_class(
        &control_with_caller("com/example/Caller"),
        "com/example/Target"
    ));

    // No captured frame → rejected (conservative; matches the pre-fix throw).
    assert!(!caller_is_same_class(
        &NativeControl::default(),
        "com/example/Target"
    ));
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

#[test]
fn annotation_class_get_annotation_returns_configured_and_default_values() {
    assert_eq!(
        run_bootstrap_int(
            "AnnotationTest.class",
            "classAnnotationConfiguredAndDefaults",
            "()I"
        ),
        1
    );
}

#[test]
fn annotation_method_and_field_get_annotation_return_values() {
    assert_eq!(
        run_bootstrap_int("AnnotationTest.class", "methodAndFieldAnnotations", "()I"),
        1
    );
}

#[test]
fn annotation_get_annotations_array_contains_runtime_annotation() {
    assert_eq!(
        run_bootstrap_int(
            "AnnotationTest.class",
            "annotationsArrayIncludesRuntimeAnnotation",
            "()I"
        ),
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
    let delta = run_bootstrap_long("TimePrimitivesTest.class", "nanoTimeDeltaAfterSleep", "()J");
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
) -> Result<Option<Slot>> {
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
    let mut owned_entries: Vec<(String, Vec<u8>)> = Vec::with_capacity(archive_entries.len() + 1);
    owned_entries.push(("META-INF/MANIFEST.MF".to_string(), manifest.into_bytes()));
    for (entry_name, entry_bytes) in archive_entries {
        owned_entries.push(((*entry_name).to_string(), entry_bytes.clone()));
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

fn allocate_url_class_loader_for_path(
    registry: &mut ClassRegistry,
    heap: &mut duke_gc::Heap,
    path: &std::path::Path,
) -> u64 {
    let url_class_path_ctx = ClassContext {
        class_name: "jdk/internal/loader/URLClassPath".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "path".to_string(),
            descriptor: "Ljava/util/ArrayList;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    let url_loader_ctx = ClassContext {
        class_name: "java/net/URLClassLoader".to_string(),
        super_class: Some("java/lang/ClassLoader".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "ucp".to_string(),
            descriptor: "Ljdk/internal/loader/URLClassPath;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    if !registry.contains("jdk/internal/loader/URLClassPath") {
        registry.register(url_class_path_ctx);
    }
    if !registry.contains("java/net/URLClassLoader") {
        registry.register(url_loader_ctx);
    }

    let path_list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
    native_arraylist_init(
        &[Slot::Reference(Some(path_list_ref))],
        heap,
        &mut Vec::new(),
        &mut NativeControl::default(),
    )
    .expect("ArrayList init for URLClassLoader.path");
    let spec_ref = heap.allocate_string(path_to_file_url(path));
    let url_ref = heap.allocate("java/net/URL".to_string(), 1);
    heap.get_mut(url_ref).expect("url object").fields[0] = Slot::Reference(Some(spec_ref));
    native_arraylist_add(
        &[
            Slot::Reference(Some(path_list_ref)),
            Slot::Reference(Some(url_ref)),
        ],
        heap,
        &mut Vec::new(),
        &mut NativeControl::default(),
    )
    .expect("ArrayList add URL");

    let ucp_ref = heap.allocate("jdk/internal/loader/URLClassPath".to_string(), 1);
    heap.get_mut(ucp_ref).expect("ucp object").fields[0] = Slot::Reference(Some(path_list_ref));

    let loader_ref = heap.allocate(
        "java/net/URLClassLoader".to_string(),
        total_instance_field_count(registry, "java/net/URLClassLoader"),
    );
    init_object_fields(registry, heap, loader_ref, "java/net/URLClassLoader");
    heap.get_mut(loader_ref).expect("loader object").fields[0] = Slot::Reference(Some(ucp_ref));
    loader_ref
}

fn allocate_url_from_spec(
    registry: &mut ClassRegistry,
    heap: &mut duke_gc::Heap,
    spec: String,
) -> u64 {
    let url_ref = heap.allocate(
        "java/net/URL".to_string(),
        total_instance_field_count(registry, "java/net/URL"),
    );
    init_object_fields(registry, heap, url_ref, "java/net/URL");
    let spec_ref = heap.allocate_string(spec);
    let init =
        match registry
            .natives_mut()
            .get_kind("java/net/URL", "<init>", "(Ljava/lang/String;)V")
        {
            Some(HandlerKind::Simple(handler)) => handler,
            other => panic!("expected URL(String) native constructor, got {other:?}"),
        };
    init(
        &[
            Slot::Reference(Some(url_ref)),
            Slot::Reference(Some(spec_ref)),
        ],
        heap,
        &mut Vec::new(),
        &mut NativeControl::default(),
    )
    .expect("URL(String) should succeed");
    url_ref
}

fn allocate_url_array(heap: &mut duke_gc::Heap, urls: &[u64]) -> u64 {
    let array_ref = heap.allocate("[Ljava/net/URL;".to_string(), urls.len());
    heap.get_mut(array_ref).expect("URL[] object").fields = urls
        .iter()
        .copied()
        .map(|url_ref| Slot::Reference(Some(url_ref)))
        .collect();
    array_ref
}

fn allocate_constructed_url_class_loader(
    registry: &mut ClassRegistry,
    heap: &mut duke_gc::Heap,
    urls_ref: u64,
) -> u64 {
    let loader_ref = heap.allocate(
        "java/net/URLClassLoader".to_string(),
        total_instance_field_count(registry, "java/net/URLClassLoader"),
    );
    init_object_fields(registry, heap, loader_ref, "java/net/URLClassLoader");
    let init = match registry.natives_mut().get_kind(
        "java/net/URLClassLoader",
        "<init>",
        "([Ljava/net/URL;)V",
    ) {
        Some(HandlerKind::Simple(handler)) => handler,
        other => panic!("expected URLClassLoader(URL[]) native constructor, got {other:?}"),
    };
    init(
        &[
            Slot::Reference(Some(loader_ref)),
            Slot::Reference(Some(urls_ref)),
        ],
        heap,
        &mut Vec::new(),
        &mut NativeControl::default(),
    )
    .expect("URLClassLoader(URL[]) should succeed");
    loader_ref
}

fn load_class_via_url_class_loader(
    boot_loader: &duke_loader::ZipLoader,
    registry: &mut ClassRegistry,
    heap: &mut duke_gc::Heap,
    loader_ref: u64,
    binary_name: &str,
) -> Result<Option<Slot>> {
    let name_ref = heap.allocate_string(binary_name.to_string());
    match registry.natives_mut().get_kind(
        "java/net/URLClassLoader",
        "loadClass",
        "(Ljava/lang/String;)Ljava/lang/Class;",
    ) {
        Some(HandlerKind::Callback(handler)) => {
            let mut ops = InterpreterCallbackOps {
                registry,
                loader: boot_loader,
            };
            handler(
                &[
                    Slot::Reference(Some(loader_ref)),
                    Slot::Reference(Some(name_ref)),
                ],
                heap,
                &mut Vec::new(),
                &mut NativeControl::default(),
                &mut ops,
            )
        }
        other => panic!("expected URLClassLoader.loadClass(String) callback, got {other:?}"),
    }
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
    let class_key_one =
        class_key_from_ref(&heap, class_one_ref).expect("loader-one mirror should carry class key");
    let class_key_two =
        class_key_from_ref(&heap, class_two_ref).expect("loader-two mirror should carry class key");

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
    use duke_classfile::CpIndex;
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
        is_native: false,
        is_abstract: false,
        instructions: instructions.into(),
        max_stack: 2,
        max_locals: 1,
        exception_table,
        pc_to_idx: Arc::new(pc_to_idx),
        line_number_table: Vec::new(),
        source_file: None,
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
fn class_for_name_uses_url_class_loader_file_urls() {
    let boot_loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let jar_path = fixtures_dir()
        .join("hello.jar")
        .canonicalize()
        .expect("canonical hello.jar");
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader_ref = allocate_url_class_loader_for_path(&mut registry, &mut heap, &jar_path);

    let binary_name_ref = heap.allocate_string("HelloWorld".to_string());
    let mut ops = InterpreterCallbackOps {
        registry: &mut registry,
        loader: &boot_loader,
    };
    let class_slot = native_class_for_name_with_loader(
        &[
            Slot::Reference(Some(binary_name_ref)),
            Slot::Int(0),
            Slot::Reference(Some(loader_ref)),
        ],
        &mut heap,
        &mut Vec::new(),
        &mut NativeControl::default(),
        &mut ops,
    )
    .expect("Class.forName should succeed")
    .expect("Class.forName should return class");
    let Slot::Reference(Some(class_ref)) = class_slot else {
        panic!("expected Class reference");
    };
    let class_key = class_key_from_ref(&heap, class_ref).expect("class key from mirror");
    assert!(
        class_key.starts_with("HelloWorld\0loader:"),
        "expected URLClassLoader class key, got {class_key:?}"
    );
}

#[test]
fn url_class_loader_load_class_reads_from_directory_url() {
    let boot_loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let classes_dir = fixtures_dir().canonicalize().expect("canonical fixtures");
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let classpath_url =
        allocate_url_from_spec(&mut registry, &mut heap, path_to_file_url(&classes_dir));
    let url_array_ref = allocate_url_array(&mut heap, &[classpath_url]);
    let loader_ref = allocate_constructed_url_class_loader(&mut registry, &mut heap, url_array_ref);

    let class_slot = load_class_via_url_class_loader(
        &boot_loader,
        &mut registry,
        &mut heap,
        loader_ref,
        "HelloWorld",
    )
    .expect("loadClass should succeed")
    .expect("loadClass should return a Class");
    let Slot::Reference(Some(class_ref)) = class_slot else {
        panic!("expected Class reference");
    };
    let class_key = class_key_from_ref(&heap, class_ref).expect("class key from mirror");
    assert!(
        class_key.starts_with("HelloWorld\0loader:"),
        "expected directory URLClassLoader class key, got {class_key:?}"
    );
}

#[test]
fn url_class_loader_load_class_reads_from_jar_url() {
    let boot_loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let jar_path = fixtures_dir()
        .join("hello.jar")
        .canonicalize()
        .expect("canonical hello.jar");
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let classpath_url =
        allocate_url_from_spec(&mut registry, &mut heap, path_to_file_url(&jar_path));
    let url_array_ref = allocate_url_array(&mut heap, &[classpath_url]);
    let loader_ref = allocate_constructed_url_class_loader(&mut registry, &mut heap, url_array_ref);

    let class_slot = load_class_via_url_class_loader(
        &boot_loader,
        &mut registry,
        &mut heap,
        loader_ref,
        "HelloWorld",
    )
    .expect("loadClass should succeed")
    .expect("loadClass should return a Class");
    let Slot::Reference(Some(class_ref)) = class_slot else {
        panic!("expected Class reference");
    };
    let class_key = class_key_from_ref(&heap, class_ref).expect("class key from mirror");
    assert!(
        class_key.starts_with("HelloWorld\0loader:"),
        "expected jar URLClassLoader class key, got {class_key:?}"
    );
}

#[test]
fn url_class_loader_instances_isolate_same_binary_name() {
    let boot_loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let jar_path = fixtures_dir()
        .join("hello.jar")
        .canonicalize()
        .expect("canonical hello.jar");
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let first_url = allocate_url_from_spec(&mut registry, &mut heap, path_to_file_url(&jar_path));
    let second_url = allocate_url_from_spec(&mut registry, &mut heap, path_to_file_url(&jar_path));
    let first_urls = allocate_url_array(&mut heap, &[first_url]);
    let second_urls = allocate_url_array(&mut heap, &[second_url]);
    let first_loader = allocate_constructed_url_class_loader(&mut registry, &mut heap, first_urls);
    let second_loader =
        allocate_constructed_url_class_loader(&mut registry, &mut heap, second_urls);

    let first_slot = load_class_via_url_class_loader(
        &boot_loader,
        &mut registry,
        &mut heap,
        first_loader,
        "HelloWorld",
    )
    .expect("first loadClass should succeed")
    .expect("first loadClass should return a Class");
    let second_slot = load_class_via_url_class_loader(
        &boot_loader,
        &mut registry,
        &mut heap,
        second_loader,
        "HelloWorld",
    )
    .expect("second loadClass should succeed")
    .expect("second loadClass should return a Class");
    let Slot::Reference(Some(first_class_ref)) = first_slot else {
        panic!("expected first Class reference");
    };
    let Slot::Reference(Some(second_class_ref)) = second_slot else {
        panic!("expected second Class reference");
    };
    let first_key = class_key_from_ref(&heap, first_class_ref).expect("first class key");
    let second_key = class_key_from_ref(&heap, second_class_ref).expect("second class key");

    assert_ne!(first_key, second_key);
    assert!(first_key.ends_with(&format!("loader:{first_loader}")));
    assert!(second_key.ends_with(&format!("loader:{second_loader}")));
}

#[test]
fn url_class_loader_load_class_missing_throws_class_not_found() {
    let boot_loader =
        duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
            .expect("open spring-boot-loader jar");
    let jar_path = fixtures_dir()
        .join("hello.jar")
        .canonicalize()
        .expect("canonical hello.jar");
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    let classpath_url =
        allocate_url_from_spec(&mut registry, &mut heap, path_to_file_url(&jar_path));
    let url_array_ref = allocate_url_array(&mut heap, &[classpath_url]);
    let loader_ref = allocate_constructed_url_class_loader(&mut registry, &mut heap, url_array_ref);

    let result = load_class_via_url_class_loader(
        &boot_loader,
        &mut registry,
        &mut heap,
        loader_ref,
        "com.example.DoesNotExist",
    );

    assert!(
        matches!(
            result,
            Err(Error::JavaException { ref class_name })
                if class_name == "java/lang/ClassNotFoundException"
        ),
        "missing class should throw ClassNotFoundException, got {result:?}"
    );
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
    let path_to_file =
        match registry
            .natives_mut()
            .get_kind("java/nio/file/Path", "toFile", "()Ljava/io/File;")
        {
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

    let get_class =
        match registry
            .natives_mut()
            .get_kind("java/lang/Object", "getClass", "()Ljava/lang/Class;")
        {
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
        ) -> Result<Option<Slot>> {
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

        fn ensure_loaded(&mut self, _class: &str) -> Result<()> {
            Ok(())
        }

        fn inspect_class(&mut self, _class: &str) -> Result<ReflectedClassInfo> {
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
        ) -> Result<Option<Slot>> {
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

        fn ensure_loaded(&mut self, _class: &str) -> Result<()> {
            Ok(())
        }

        fn inspect_class(&mut self, _class: &str) -> Result<ReflectedClassInfo> {
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
        ) -> Result<Option<Slot>> {
            Ok(None)
        }

        fn ensure_loaded(&mut self, class: &str) -> Result<()> {
            self.loaded.push(class.to_string());
            Ok(())
        }

        fn inspect_class(&mut self, _class: &str) -> Result<ReflectedClassInfo> {
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

/// Regression: a `Class.forName(name, false, cl)` availability probe for an absent
/// class must surface a catchable `ClassNotFoundException`, even when computing the
/// class identity key would itself raise `ClassNotFound`. The native previously
/// computed the key eagerly (with `?`) alongside the load attempt, so a missing
/// class made the key lookup fail first and propagate a fatal `ClassNotFound`
/// instead of the mapped Java exception. Commons-logging's backend probing relies
/// on the exception being catchable (e.g. the `SLF4JProvider`/`Log4jApiLogFactory`
/// availability checks in `LogFactory.newStandardFactory`).
#[test]
fn native_class_for_name_missing_class_throws_even_when_key_lookup_fails() {
    #[derive(Default)]
    struct MissingKeyOps;

    impl CallbackOps for MissingKeyOps {
        fn invoke(
            &mut self,
            _heap: &mut duke_gc::Heap,
            _output: &mut dyn Write,
            _class: &str,
            _method: &str,
            _descriptor: &str,
            _args: Vec<Slot>,
        ) -> Result<Option<Slot>> {
            Ok(None)
        }

        fn ensure_loaded(&mut self, class: &str) -> Result<()> {
            Err(Error::ClassNotFound {
                name: class.to_string(),
            })
        }

        // Mirrors the real registry: resolving the identity key for an unloaded
        // class raises `ClassNotFound`. This must NOT escape as a fatal error.
        fn class_key_for_loaded_class(&mut self, class: &str) -> Result<String> {
            Err(Error::ClassNotFound {
                name: class.to_string(),
            })
        }

        fn inspect_class(&mut self, _class: &str) -> Result<ReflectedClassInfo> {
            unreachable!("inspect_class should not be used")
        }
    }

    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let binary_name_ref =
        heap.allocate_string("org.apache.logging.slf4j.SLF4JProvider".to_string());
    let mut ops = MissingKeyOps;

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
    );

    assert!(
        matches!(
            result,
            Err(Error::JavaException { ref class_name })
                if class_name == "java/lang/ClassNotFoundException"
        ),
        "missing class must throw catchable ClassNotFoundException, not a fatal error: {result:?}"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn native_class_for_name_with_loader_uses_loader_archive_not_global_default_code_source() {
    let loader = duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
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
            Err(Error::JavaException { ref class_name })
                if class_name == "java/lang/ClassNotFoundException"
        ),
        "wrong launched loader should not fall back to registry default code source: {result:?}"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn native_reflect_method_invoke_runs_boot_archive_main() {
    let loader = duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
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
        let string_array_class_ref =
            allocate_class_object(&mut heap, "[Ljava/lang/String;").expect("allocate array Class");
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
    let method = reflected_method_handle(&heap, method_ref).expect("read reflected method handle");
    let invoke_arg_slots = reflection_array_elements(&heap, Slot::Reference(Some(invoke_args_ref)))
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
        ) -> Result<Option<Slot>> {
            Ok(None)
        }

        fn ensure_loaded(&mut self, _class: &str) -> Result<()> {
            Ok(())
        }

        fn inspect_class(&mut self, class: &str) -> Result<ReflectedClassInfo> {
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
                        annotations: Vec::new(),
                        annotation_default: None,
                    },
                    ReflectedMethodInfo {
                        name: "main".to_string(),
                        descriptor: "()V".to_string(),
                        is_public: true,
                        is_static: true,
                        annotations: Vec::new(),
                        annotation_default: None,
                    },
                ],
                fields: Vec::new(),
                access_flags: 0x0001,
                annotations: Vec::new(),
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
fn native_thread_current_thread_returns_stable_thread_object() {
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    // Bootstrap registers the synthetic `java/lang/Thread` (with the GC-rooted
    // `$dukeMainThread` static slot) and seeds the stable main-thread identity.
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();

    let call_current_thread = |registry: &mut ClassRegistry, heap: &mut duke_gc::Heap| -> Slot {
        let mut sink: Vec<u8> = Vec::new();
        let mut ops = InterpreterCallbackOps {
            registry,
            loader: &loader,
        };
        native_thread_current_thread(
            &[],
            heap,
            &mut sink,
            &mut NativeControl::default(),
            &mut ops,
        )
        .expect("Thread.currentThread should succeed")
        .expect("Thread.currentThread should return a thread")
    };

    let first = call_current_thread(&mut registry, &mut heap);
    let second = call_current_thread(&mut registry, &mut heap);

    // Wave-9 fix (C): the identity must be STABLE — both calls return the SAME
    // heap ref, so `ReentrantLock`'s acquire-owner == release-owner check balances.
    assert_eq!(
        first, second,
        "Thread.currentThread() must return a stable identity across calls"
    );

    let Slot::Reference(Some(thread_ref)) = first else {
        panic!("expected Thread reference");
    };
    let thread = heap.get(thread_ref).unwrap();
    assert_eq!(thread.class_name, "java/lang/Thread");
    assert_eq!(thread.fields[THREAD_ID_SLOT], Slot::Int(-1));
}

#[test]
fn thread_current_thread_identity_is_stable_through_invokestatic() {
    // End-to-end via real `invokestatic` dispatch: `ThreadIdentityProbe.sameIdentity`
    // calls `Thread.currentThread()` twice and returns 1 iff `a == b` (reference
    // equality). Wave-9 fix (C) makes the identity stable, so this returns 1.
    assert_eq!(
        run_bootstrap_int("ThreadIdentityProbe.class", "sameIdentity", "()I"),
        1
    );
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
    let desired_assertion_status =
        match registry
            .natives_mut()
            .get_kind("java/lang/Class", "desiredAssertionStatus", "()Z")
        {
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
            Error::AmbiguousClassName { ref name, ref matches }
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
            Error::AmbiguousClassName { ref name, ref matches }
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
            (1, Instruction::Instanceof(duke_classfile::CpIndex(1))),
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
            (1, Instruction::Checkcast(duke_classfile::CpIndex(1))),
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
        matches!(&err, Error::JavaException { class_name } if class_name == "java/lang/ClassCastException"),
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
            Error::JavaException { ref class_name } if class_name == &class_key_two
        ),
        "expected uncaught JavaException from loader-two object, got {err:?}"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn native_class_get_class_loader_returns_boot_launched_loader_for_loaded_app_class() {
    let loader = duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
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
        let handler = match registry
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
    let entry_class_key =
        class_key_from_ref(&heap, entry_class_ref).expect("class mirror should carry a class key");

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
    let loader = duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
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
    let loader = duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
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
    let loader = duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
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
    let loader = duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
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
    let loader = duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
        .expect("open spring-boot-loader jar");
    let hello_world = std::fs::read(fixtures_dir().join("HelloWorld.class"))
        .expect("read HelloWorld.class fixture");
    let filler_nested_jar = build_test_multi_entry_zip(&[]);
    let mut archive_entries = vec![("BOOT-INF/classes/HelloWorld.class".to_string(), hello_world)];
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
    let loader = duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
        .expect("open spring-boot-loader jar");
    let hello_world = std::fs::read(fixtures_dir().join("HelloWorld.class"))
        .expect("read HelloWorld.class fixture");
    let filler_nested_jar = build_test_multi_entry_zip(&[]);
    let mut archive_entries = vec![("BOOT-INF/classes/HelloWorld.class".to_string(), hello_world)];
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
    let loader = duke_loader::ZipLoader::open(&repo_root().join("spring-boot-loader-3.5.12.jar"))
        .expect("open spring-boot-loader jar");
    let hello_world = std::fs::read(fixtures_dir().join("HelloWorld.class"))
        .expect("read HelloWorld.class fixture");
    let filler_nested_jar = build_test_multi_entry_zip(&[]);
    let mut archive_entries = vec![("BOOT-INF/classes/HelloWorld.class".to_string(), hello_world)];
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

// ---- Phase 33: HashSet iteration, LinkedList, HashMap.forEach, Integer string conv ----

#[test]
fn hashset_for_each_counts_elements() {
    assert_eq!(
        run_bootstrap_int("HashSetIterTest.class", "testHashSetForEach", "()I"),
        3,
    );
}

#[test]
fn hashset_keyset_for_each_counts_entries() {
    assert_eq!(
        run_bootstrap_int("HashSetIterTest.class", "testKeySetForEach", "()I"),
        2,
    );
}

#[test]
fn hashset_for_each_sums_integers() {
    assert_eq!(
        run_bootstrap_int("HashSetIterTest.class", "testSumValues", "()I"),
        60,
    );
}

#[test]
fn linked_list_size() {
    assert_eq!(
        run_bootstrap_int("LinkedListTest.class", "testSize", "()I"),
        3,
    );
}

#[test]
fn linked_list_peek_first() {
    assert_eq!(
        run_bootstrap_int("LinkedListTest.class", "testPeekFirst", "()I"),
        10,
    );
}

#[test]
fn linked_list_remove_first() {
    assert_eq!(
        run_bootstrap_int("LinkedListTest.class", "testRemoveFirst", "()I"),
        1,
    );
}

#[test]
fn linked_list_add_first() {
    assert_eq!(
        run_bootstrap_int("LinkedListTest.class", "testAddFirst", "()I"),
        1,
    );
}

#[test]
fn linked_list_peek_last() {
    assert_eq!(
        run_bootstrap_int("LinkedListTest.class", "testPeekLast", "()I"),
        9,
    );
}

#[test]
fn linked_list_poll() {
    assert_eq!(
        run_bootstrap_int("LinkedListTest.class", "testPoll", "()I"),
        101,
    );
}

#[test]
fn integer_to_binary_string_length() {
    assert_eq!(
        run_bootstrap_int("IntegerStringConvTest.class", "testToBinaryString", "()I"),
        4,
    );
}

#[test]
fn integer_to_hex_string_length() {
    assert_eq!(
        run_bootstrap_int("IntegerStringConvTest.class", "testToHexString", "()I"),
        2,
    );
}

#[test]
fn integer_to_octal_string_length() {
    assert_eq!(
        run_bootstrap_int("IntegerStringConvTest.class", "testToOctalString", "()I"),
        2,
    );
}

#[test]
fn integer_to_binary_string_one() {
    assert_eq!(
        run_bootstrap_int(
            "IntegerStringConvTest.class",
            "testToBinaryStringOne",
            "()I"
        ),
        1,
    );
}

#[test]
fn hashmap_for_each_count() {
    assert_eq!(
        run_bootstrap_int("HashMapForEachTest.class", "testForEachCount", "()I"),
        3,
    );
}

#[test]
fn hashmap_for_each_sum_values() {
    assert_eq!(
        run_bootstrap_int("HashMapForEachTest.class", "testForEachSumValues", "()I"),
        30,
    );
}

// ---- Phase 34: TreeMap, Stack, Comparator ----

#[test]
fn treemap_size() {
    assert_eq!(run_bootstrap_int("TreeMapTest.class", "testSize", "()I"), 3,);
}

#[test]
fn treemap_get() {
    assert_eq!(run_bootstrap_int("TreeMapTest.class", "testGet", "()I"), 42,);
}

#[test]
fn treemap_first_key() {
    assert_eq!(
        run_bootstrap_int("TreeMapTest.class", "testFirstKey", "()I"),
        1,
    );
}

#[test]
fn treemap_last_key() {
    assert_eq!(
        run_bootstrap_int("TreeMapTest.class", "testLastKey", "()I"),
        1,
    );
}

#[test]
fn treemap_contains_key() {
    assert_eq!(
        run_bootstrap_int("TreeMapTest.class", "testContainsKey", "()I"),
        1,
    );
}

#[test]
fn stack_push_pop() {
    assert_eq!(
        run_bootstrap_int("StackTest.class", "testPushPop", "()I"),
        3,
    );
}

#[test]
fn stack_peek() {
    assert_eq!(run_bootstrap_int("StackTest.class", "testPeek", "()I"), 20,);
}

#[test]
fn stack_size() {
    assert_eq!(run_bootstrap_int("StackTest.class", "testSize", "()I"), 2,);
}

#[test]
fn stack_empty() {
    assert_eq!(run_bootstrap_int("StackTest.class", "testEmpty", "()I"), 1,);
}

#[test]
fn stack_not_empty() {
    assert_eq!(
        run_bootstrap_int("StackTest.class", "testNotEmpty", "()I"),
        1,
    );
}

#[test]
fn comparator_natural_order_sort() {
    assert_eq!(
        run_bootstrap_int("ComparatorTest.class", "testNaturalOrderSort", "()I"),
        5,
    );
}

#[test]
fn comparator_reverse_order_sort() {
    assert_eq!(
        run_bootstrap_int("ComparatorTest.class", "testReverseOrderSort", "()I"),
        3,
    );
}

#[test]
fn comparator_comparing_int_sort() {
    assert_eq!(
        run_bootstrap_int(
            "ComparatorTest.class",
            "testCollectionsSortWithComparator",
            "()I"
        ),
        1,
    );
}

// ---- Phase 35: LinkedHashMap, TreeSet, Collections.min/max, String extras ----

#[test]
fn linked_hashmap_size() {
    assert_eq!(
        run_bootstrap_int("LinkedHashMapTest.class", "testSize", "()I"),
        3,
    );
}

#[test]
fn linked_hashmap_get() {
    assert_eq!(
        run_bootstrap_int("LinkedHashMapTest.class", "testGet", "()I"),
        42,
    );
}

#[test]
fn linked_hashmap_contains_key() {
    assert_eq!(
        run_bootstrap_int("LinkedHashMapTest.class", "testContainsKey", "()I"),
        1,
    );
}

#[test]
fn linked_hashmap_insertion_order() {
    assert_eq!(
        run_bootstrap_int("LinkedHashMapTest.class", "testInsertionOrder", "()I"),
        5,
    );
}

#[test]
fn treeset_size() {
    assert_eq!(run_bootstrap_int("TreeSetTest.class", "testSize", "()I"), 3,);
}

#[test]
fn treeset_first() {
    assert_eq!(
        run_bootstrap_int("TreeSetTest.class", "testFirst", "()I"),
        1,
    );
}

#[test]
fn treeset_last() {
    assert_eq!(run_bootstrap_int("TreeSetTest.class", "testLast", "()I"), 1,);
}

#[test]
fn treeset_contains() {
    assert_eq!(
        run_bootstrap_int("TreeSetTest.class", "testContains", "()I"),
        1,
    );
}

#[test]
fn treeset_no_duplicates() {
    assert_eq!(
        run_bootstrap_int("TreeSetTest.class", "testNoDuplicates", "()I"),
        2,
    );
}

#[test]
fn collections_min_int() {
    assert_eq!(
        run_bootstrap_int("CollectionsMinMaxTest.class", "testMinInt", "()I"),
        1,
    );
}

#[test]
fn collections_max_int() {
    assert_eq!(
        run_bootstrap_int("CollectionsMinMaxTest.class", "testMaxInt", "()I"),
        4,
    );
}

#[test]
fn collections_min_string() {
    assert_eq!(
        run_bootstrap_int("CollectionsMinMaxTest.class", "testMinString", "()I"),
        5,
    );
}

#[test]
fn collections_shuffle_preserves_size() {
    assert_eq!(
        run_bootstrap_int("CollectionsMinMaxTest.class", "testShuffle", "()I"),
        3,
    );
}

#[test]
fn string_to_char_array_length() {
    assert_eq!(
        run_bootstrap_int("StringCharsTest.class", "testToCharArrayLength", "()I"),
        5,
    );
}

#[test]
fn string_manual_char_iteration() {
    assert_eq!(
        run_bootstrap_int("StringCharsTest.class", "testManualCharIteration", "()I"),
        294,
    );
}

#[test]
fn string_code_point_at() {
    assert_eq!(
        run_bootstrap_int("StringCharsTest.class", "testCodePointAt", "()I"),
        65,
    );
}

#[test]
fn string_compare_to_ordering() {
    assert_eq!(
        run_bootstrap_int("StringCharsTest.class", "testCompareTo", "()I"),
        1,
    );
}

#[test]
fn string_value_of_char() {
    assert_eq!(
        run_bootstrap_int("StringCharsTest.class", "testValueOfChar", "()I"),
        1,
    );
}

// ---- Phase 36: Stream, ArrayDeque ----

#[test]
fn stream_of_count() {
    assert_eq!(
        run_bootstrap_int("StreamTest.class", "testStreamOfCount", "()I"),
        3,
    );
}

#[test]
fn stream_filter() {
    assert_eq!(
        run_bootstrap_int("StreamTest.class", "testStreamFilter", "()I"),
        2,
    );
}

#[test]
fn stream_map() {
    assert_eq!(
        run_bootstrap_int("StreamTest.class", "testStreamMap", "()I"),
        5,
    );
}

#[test]
fn stream_for_each() {
    assert_eq!(
        run_bootstrap_int("StreamTest.class", "testStreamForEach", "()I"),
        6,
    );
}

#[test]
fn stream_collection_stream() {
    assert_eq!(
        run_bootstrap_int("StreamTest.class", "testCollectionStream", "()I"),
        3,
    );
}

#[test]
fn stream_to_list() {
    assert_eq!(
        run_bootstrap_int("StreamTest.class", "testStreamToList", "()I"),
        2,
    );
}

#[test]
fn stream_distinct() {
    assert_eq!(
        run_bootstrap_int("StreamTest.class", "testStreamDistinct", "()I"),
        3,
    );
}

#[test]
fn stream_map_to_length() {
    assert_eq!(
        run_bootstrap_int("StreamTest.class", "testStreamMapToLength", "()I"),
        2,
    );
}

#[test]
fn arraydeque_push_pop() {
    assert_eq!(
        run_bootstrap_int("ArrayDequeTest.class", "testPushPop", "()I"),
        3,
    );
}

#[test]
fn arraydeque_offer_poll() {
    assert_eq!(
        run_bootstrap_int("ArrayDequeTest.class", "testOfferPoll", "()I"),
        10,
    );
}

#[test]
fn arraydeque_peek() {
    assert_eq!(
        run_bootstrap_int("ArrayDequeTest.class", "testPeek", "()I"),
        5,
    );
}

#[test]
fn arraydeque_size() {
    assert_eq!(
        run_bootstrap_int("ArrayDequeTest.class", "testSize", "()I"),
        2,
    );
}

#[test]
fn arraydeque_is_empty() {
    assert_eq!(
        run_bootstrap_int("ArrayDequeTest.class", "testIsEmpty", "()I"),
        1,
    );
}

// ---- Phase 37: Stream extensions, PriorityQueue ----

#[test]
fn stream_sorted() {
    assert_eq!(
        run_bootstrap_int("StreamExtTest.class", "testSorted", "()I"),
        1,
    );
}

#[test]
fn stream_any_match() {
    assert_eq!(
        run_bootstrap_int("StreamExtTest.class", "testAnyMatch", "()I"),
        1,
    );
}

#[test]
fn stream_all_match() {
    assert_eq!(
        run_bootstrap_int("StreamExtTest.class", "testAllMatch", "()I"),
        1,
    );
}

#[test]
fn stream_none_match() {
    assert_eq!(
        run_bootstrap_int("StreamExtTest.class", "testNoneMatch", "()I"),
        1,
    );
}

#[test]
fn stream_find_first() {
    assert_eq!(
        run_bootstrap_int("StreamExtTest.class", "testFindFirst", "()I"),
        1,
    );
}

#[test]
fn stream_find_first_value() {
    assert_eq!(
        run_bootstrap_int("StreamExtTest.class", "testFindFirstValue", "()I"),
        5,
    );
}

#[test]
fn stream_reduce() {
    assert_eq!(
        run_bootstrap_int("StreamExtTest.class", "testReduce", "()I"),
        15,
    );
}

#[test]
fn stream_collectors_joining() {
    assert_eq!(
        run_bootstrap_int("StreamExtTest.class", "testCollectorsJoining", "()I"),
        7,
    );
}

#[test]
fn stream_ext_to_list() {
    assert_eq!(
        run_bootstrap_int("StreamExtTest.class", "testToList", "()I"),
        2,
    );
}

#[test]
fn stream_filter_count() {
    assert_eq!(
        run_bootstrap_int("StreamExtTest.class", "testStreamCount", "()I"),
        2,
    );
}

#[test]
fn priorityqueue_poll_order() {
    assert_eq!(
        run_bootstrap_int("PriorityQueueTest.class", "testPollOrder", "()I"),
        1,
    );
}

#[test]
fn priorityqueue_peek_min() {
    assert_eq!(
        run_bootstrap_int("PriorityQueueTest.class", "testPeekMin", "()I"),
        2,
    );
}

#[test]
fn priorityqueue_size() {
    assert_eq!(
        run_bootstrap_int("PriorityQueueTest.class", "testSize", "()I"),
        3,
    );
}

#[test]
fn priorityqueue_is_empty() {
    assert_eq!(
        run_bootstrap_int("PriorityQueueTest.class", "testIsEmpty", "()I"),
        1,
    );
}

#[test]
fn priorityqueue_poll_all() {
    assert_eq!(
        run_bootstrap_int("PriorityQueueTest.class", "testPollAll", "()I"),
        9,
    );
}

// ---- Phase 38: Random, StringBuffer, StringJoiner, String.lines ----

#[test]
fn random_range() {
    assert_eq!(
        run_bootstrap_int("Phase38Test.class", "testRandomRange", "()I"),
        1,
    );
}

#[test]
fn random_seed_deterministic() {
    assert_eq!(
        run_bootstrap_int("Phase38Test.class", "testRandomSeedDeterministic", "()I"),
        1,
    );
}

#[test]
fn random_boolean() {
    assert_eq!(
        run_bootstrap_int("Phase38Test.class", "testRandomBoolean", "()I"),
        1,
    );
}

#[test]
fn random_next_int() {
    assert_eq!(
        run_bootstrap_int("Phase38Test.class", "testRandomNextInt", "()I"),
        1,
    );
}

#[test]
fn random_next_double() {
    assert_eq!(
        run_bootstrap_int("Phase38Test.class", "testRandomNextDouble", "()I"),
        1,
    );
}

#[test]
fn random_next_long() {
    assert_eq!(
        run_bootstrap_int("Phase38Test.class", "testRandomNextLong", "()I"),
        1,
    );
}

#[test]
fn stringbuffer_append() {
    assert_eq!(
        run_bootstrap_int("Phase38Test.class", "testStringBufferAppend", "()I"),
        11,
    );
}

#[test]
fn stringbuffer_init() {
    assert_eq!(
        run_bootstrap_int("Phase38Test.class", "testStringBufferInit", "()I"),
        2,
    );
}

#[test]
fn stringbuffer_tostring() {
    assert_eq!(
        run_bootstrap_int("Phase38Test.class", "testStringBufferToString", "()I"),
        1,
    );
}

#[test]
fn stringjoiner_basic() {
    assert_eq!(
        run_bootstrap_int("Phase38Test.class", "testStringJoinerBasic", "()I"),
        1,
    );
}

#[test]
fn stringjoiner_empty() {
    assert_eq!(
        run_bootstrap_int("Phase38Test.class", "testStringJoinerEmpty", "()I"),
        0,
    );
}

#[test]
fn stringjoiner_prefix_suffix() {
    assert_eq!(
        run_bootstrap_int(
            "Phase38Test.class",
            "testStringJoinerWithPrefixSuffix",
            "()I"
        ),
        1,
    );
}

#[test]
fn stringjoiner_set_empty_value() {
    assert_eq!(
        run_bootstrap_int("Phase38Test.class", "testStringJoinerSetEmptyValue", "()I"),
        1,
    );
}

#[test]
fn stringjoiner_length() {
    assert_eq!(
        run_bootstrap_int("Phase38Test.class", "testStringJoinerLength", "()I"),
        4,
    );
}

#[test]
fn string_lines_count() {
    assert_eq!(
        run_bootstrap_int("Phase38Test.class", "testStringLines", "()I"),
        3,
    );
}

#[test]
fn string_lines_join() {
    assert_eq!(
        run_bootstrap_int("Phase38Test.class", "testStringLinesJoin", "()I"),
        1,
    );
}

// ---- Phase 39: regex Pattern/Matcher, Optional.map/filter/ifPresent/orElseGet,
//                HashMap.compute/merge ----

#[test]
fn pattern_matches_static() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testPatternMatchesStatic", "()I"),
        1,
    );
}

#[test]
fn pattern_matches_fails() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testPatternMatchesFails", "()I"),
        0,
    );
}

#[test]
fn matcher_find() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testMatcherFind", "()I"),
        1,
    );
}

#[test]
fn matcher_group() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testMatcherGroup", "()I"),
        1,
    );
}

#[test]
fn matcher_find_all() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testMatcherFindAll", "()I"),
        3,
    );
}

#[test]
fn matcher_matches() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testMatcherMatches", "()I"),
        1,
    );
}

#[test]
fn matcher_start() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testMatcherStart", "()I"),
        3,
    );
}

#[test]
fn matcher_end() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testMatcherEnd", "()I"),
        6,
    );
}

#[test]
fn matcher_replace_all() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testMatcherReplaceAll", "()I"),
        1,
    );
}

#[test]
fn matcher_replace_first() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testMatcherReplaceFirst", "()I"),
        1,
    );
}

#[test]
fn regex_flags_fixture() {
    assert_eq!(
        run_bootstrap_int("RegexFlagsTest.class", "runAll", "()I"),
        6
    );
}

#[test]
fn regex_split_fixture() {
    assert_eq!(
        run_bootstrap_int("RegexSplitTest.class", "runAll", "()I"),
        4
    );
}

#[test]
fn regex_group_count_fixture() {
    assert_eq!(
        run_bootstrap_int("RegexGroupCountTest.class", "runAll", "()I"),
        3,
    );
}

#[test]
fn regex_named_groups_fixture() {
    assert_eq!(
        run_bootstrap_int("RegexNamedGroupsTest.class", "runAll", "()I"),
        4,
    );
}

#[test]
fn regex_reset_fixture() {
    assert_eq!(
        run_bootstrap_int("RegexResetTest.class", "runAll", "()I"),
        2
    );
}

#[test]
fn regex_append_fixture() {
    assert_eq!(
        run_bootstrap_int("RegexAppendTest.class", "runAll", "()I"),
        1
    );
}

#[test]
fn regex_backref_fixture() {
    assert_eq!(
        run_bootstrap_int("RegexBackrefTest.class", "runAll", "()I"),
        2
    );
}

#[test]
fn string_matches_regex() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testStringMatches", "()I"),
        1,
    );
}

#[test]
fn string_matches_full() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testStringMatchesFull", "()I"),
        1,
    );
}

#[test]
fn string_replace_all_regex() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testStringReplaceAll", "()I"),
        1,
    );
}

#[test]
fn string_replace_first_regex() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testStringReplaceFirst", "()I"),
        1,
    );
}

#[test]
fn string_split_regex() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testStringSplitRegex", "()I"),
        4,
    );
}

#[test]
fn optional_map() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testOptionalMap", "()I"),
        5,
    );
}

#[test]
fn optional_map_empty() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testOptionalMapEmpty", "()I"),
        0,
    );
}

#[test]
fn optional_filter() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testOptionalFilter", "()I"),
        1,
    );
}

#[test]
fn optional_filter_drop() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testOptionalFilterDrop", "()I"),
        1,
    );
}

#[test]
fn optional_if_present() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testOptionalIfPresent", "()I"),
        5,
    );
}

#[test]
fn optional_or_else_get() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testOptionalOrElseGet", "()I"),
        7,
    );
}

#[test]
fn optional_or_else_get_present() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testOptionalOrElseGetPresent", "()I"),
        2,
    );
}

#[test]
fn hashmap_compute() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testHashMapCompute", "()I"),
        11,
    );
}

#[test]
fn hashmap_compute_absent() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testHashMapComputeAbsent", "()I"),
        42,
    );
}

#[test]
fn hashmap_merge() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testHashMapMerge", "()I"),
        8,
    );
}

#[test]
fn hashmap_merge_absent() {
    assert_eq!(
        run_bootstrap_int("Phase39Test.class", "testHashMapMergeAbsent", "()I"),
        42,
    );
}

// ---- Phase 40: IntStream, Stream.limit/skip/flatMap ----

#[test]
fn test_int_stream_range_sum() {
    assert_eq!(
        run_bootstrap_int("Phase40Test.class", "testIntStreamRangeSum", "()I"),
        15,
    );
}

#[test]
fn test_int_stream_range_closed_sum() {
    assert_eq!(
        run_bootstrap_int("Phase40Test.class", "testIntStreamRangeClosedSum", "()I"),
        15,
    );
}

#[test]
fn test_int_stream_range_count() {
    assert_eq!(
        run_bootstrap_int("Phase40Test.class", "testIntStreamRangeCount", "()I"),
        10,
    );
}

#[test]
fn test_int_stream_range_closed_count() {
    assert_eq!(
        run_bootstrap_int("Phase40Test.class", "testIntStreamRangeClosedCount", "()I"),
        5,
    );
}

#[test]
fn test_int_stream_filter() {
    assert_eq!(
        run_bootstrap_int("Phase40Test.class", "testIntStreamFilter", "()I"),
        5,
    );
}

#[test]
fn test_int_stream_map() {
    assert_eq!(
        run_bootstrap_int("Phase40Test.class", "testIntStreamMap", "()I"),
        14,
    );
}

#[test]
fn test_int_stream_for_each_count() {
    assert_eq!(
        run_bootstrap_int("Phase40Test.class", "testIntStreamForEachCount", "()I"),
        10,
    );
}

#[test]
fn test_int_stream_min() {
    assert_eq!(
        run_bootstrap_int("Phase40Test.class", "testIntStreamMin", "()I"),
        1,
    );
}

#[test]
fn test_int_stream_max() {
    assert_eq!(
        run_bootstrap_int("Phase40Test.class", "testIntStreamMax", "()I"),
        8,
    );
}

#[test]
fn test_int_stream_average() {
    assert_eq!(
        run_bootstrap_int("Phase40Test.class", "testIntStreamAverage", "()I"),
        2,
    );
}

#[test]
fn test_int_stream_to_array() {
    assert_eq!(
        run_bootstrap_int("Phase40Test.class", "testIntStreamToArray", "()I"),
        6,
    );
}

#[test]
fn test_int_stream_boxed() {
    assert_eq!(
        run_bootstrap_int("Phase40Test.class", "testIntStreamBoxed", "()I"),
        3,
    );
}

#[test]
fn test_int_stream_of() {
    assert_eq!(
        run_bootstrap_int("Phase40Test.class", "testIntStreamOf", "()I"),
        60,
    );
}

#[test]
fn test_stream_limit() {
    assert_eq!(
        run_bootstrap_int("Phase40Test.class", "testStreamLimit", "()I"),
        3,
    );
}

#[test]
fn test_stream_skip() {
    assert_eq!(
        run_bootstrap_int("Phase40Test.class", "testStreamSkip", "()I"),
        3,
    );
}

#[test]
fn test_stream_skip_limit() {
    assert_eq!(
        run_bootstrap_int("Phase40Test.class", "testStreamSkipLimit", "()I"),
        3,
    );
}

#[test]
fn test_stream_flat_map() {
    assert_eq!(
        run_bootstrap_int("Phase40Test.class", "testStreamFlatMap", "()I"),
        4,
    );
}

#[test]
fn test_int_stream_map_to_obj() {
    assert_eq!(
        run_bootstrap_int("Phase40Test.class", "testIntStreamMapToObj", "()I"),
        3,
    );
}

// ---- Phase 41: String.format extensions, stream methods, nCopies, chars ----

#[test]
fn test_format_width() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testFormatWidth", "()I"),
        1,
    );
}

#[test]
fn test_format_left_align() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testFormatLeftAlign", "()I"),
        1,
    );
}

#[test]
fn test_format_boolean_specifier() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testFormatBooleanSpecifier", "()I"),
        1,
    );
}

#[test]
fn test_format_char_specifier() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testFormatCharSpecifier", "()I"),
        1,
    );
}

#[test]
fn test_format_octal_specifier() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testFormatOctalSpecifier", "()I"),
        1,
    );
}

#[test]
fn test_format_scientific() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testFormatScientific", "()I"),
        1,
    );
}

#[test]
fn test_format_zero_pad() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testFormatZeroPad", "()I"),
        1,
    );
}

#[test]
fn test_format_plus() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testFormatPlus", "()I"),
        1,
    );
}

#[test]
fn test_hashset_stream() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testHashSetStream", "()I"),
        3,
    );
}

#[test]
fn test_linked_list_stream() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testLinkedListStream", "()I"),
        2,
    );
}

#[test]
fn test_parse_int_hex() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testParseIntHex", "()I"),
        255,
    );
}

#[test]
fn test_parse_int_binary() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testParseIntBinary", "()I"),
        10,
    );
}

#[test]
fn test_parse_int_octal() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testParseIntOctal", "()I"),
        15,
    );
}

#[test]
fn test_parse_long_hex() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testParseLongHex", "()I"),
        31,
    );
}

#[test]
fn test_int_to_hex_string() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testIntToHexString", "()I"),
        1,
    );
}

#[test]
fn test_int_to_binary_string() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testIntToBinaryString", "()I"),
        1,
    );
}

#[test]
fn test_int_to_octal_string() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testIntToOctalString", "()I"),
        1,
    );
}

#[test]
fn test_collections_n_copies() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testCollectionsNCopies", "()I"),
        3,
    );
}

#[test]
fn test_collections_n_copies_content() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testCollectionsNCopiesContent", "()I"),
        1,
    );
}

#[test]
fn test_string_chars() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testStringChars", "()I"),
        5,
    );
}

#[test]
fn test_string_chars_sum() {
    assert_eq!(
        run_bootstrap_int("Phase41Test.class", "testStringCharsSum", "()I"),
        294,
    );
}

// ---- Phase 42 tests ----

#[test]
fn test_collectors_counting() {
    assert_eq!(
        run_bootstrap_int("Phase42Test.class", "testCollectorsCounting", "()I"),
        3,
    );
}

#[test]
fn test_grouping_by_size() {
    assert_eq!(
        run_bootstrap_int("Phase42Test.class", "testGroupingBySize", "()I"),
        3,
    );
}

#[test]
fn test_grouping_by_count() {
    assert_eq!(
        run_bootstrap_int("Phase42Test.class", "testGroupingByCount", "()I"),
        2,
    );
}

#[test]
fn test_stream_peek() {
    assert_eq!(
        run_bootstrap_int("Phase42Test.class", "testStreamPeek", "()I"),
        6,
    );
}

#[test]
fn test_stream_to_array() {
    assert_eq!(
        run_bootstrap_int("Phase42Test.class", "testStreamToArray", "()I"),
        3,
    );
}

#[test]
fn test_arrays_stream_int() {
    assert_eq!(
        run_bootstrap_int("Phase42Test.class", "testArraysStreamInt", "()I"),
        15,
    );
}

#[test]
fn test_arrays_stream_int_filter() {
    assert_eq!(
        run_bootstrap_int("Phase42Test.class", "testArraysStreamIntFilter", "()I"),
        2,
    );
}

#[test]
fn test_math_random() {
    assert_eq!(
        run_bootstrap_int("Phase42Test.class", "testMathRandom", "()I"),
        1,
    );
}

#[test]
fn test_comparator_comparing() {
    assert_eq!(
        run_bootstrap_int("Phase42Test.class", "testComparatorComparing", "()I"),
        1,
    );
}

#[test]
fn test_hash_map_for_each() {
    assert_eq!(
        run_bootstrap_int("Phase42Test.class", "testHashMapForEach", "()I"),
        6,
    );
}

#[test]
fn test_string_join_list() {
    assert_eq!(
        run_bootstrap_int("Phase42Test.class", "testStringJoinList", "()I"),
        1,
    );
}

#[test]
fn test_unmodifiable_list() {
    assert_eq!(
        run_bootstrap_int("Phase42Test.class", "testUnmodifiableList", "()I"),
        2,
    );
}

#[test]
fn test_integer_compare() {
    assert_eq!(
        run_bootstrap_int("Phase42Test.class", "testIntegerCompare", "()I"),
        1,
    );
}

#[test]
fn test_integer_compare_equal() {
    assert_eq!(
        run_bootstrap_int("Phase42Test.class", "testIntegerCompareEqual", "()I"),
        0,
    );
}

#[test]
fn test_long_compare() {
    assert_eq!(
        run_bootstrap_int("Phase42Test.class", "testLongCompare", "()I"),
        1,
    );
}

// ---- Phase 43 tests ----

#[test]
fn test_stream_map_to_int() {
    assert_eq!(
        run_bootstrap_int("Phase43Test.class", "testStreamMapToInt", "()I"),
        10,
    );
}

#[test]
fn test_stream_map_to_int_max() {
    assert_eq!(
        run_bootstrap_int("Phase43Test.class", "testStreamMapToIntMax", "()I"),
        5,
    );
}

#[test]
fn test_collectors_to_set() {
    assert_eq!(
        run_bootstrap_int("Phase43Test.class", "testCollectorsToSet", "()I"),
        3,
    );
}

#[test]
fn test_collectors_to_map() {
    assert_eq!(
        run_bootstrap_int("Phase43Test.class", "testCollectorsToMap", "()I"),
        3,
    );
}

#[test]
fn test_collectors_to_map_get() {
    assert_eq!(
        run_bootstrap_int("Phase43Test.class", "testCollectorsToMapGet", "()I"),
        2,
    );
}

#[test]
fn test_stream_min_comparator() {
    assert_eq!(
        run_bootstrap_int("Phase43Test.class", "testStreamMinComparator", "()I"),
        5,
    );
}

#[test]
fn test_stream_max_comparator() {
    assert_eq!(
        run_bootstrap_int("Phase43Test.class", "testStreamMaxComparator", "()I"),
        6,
    );
}

#[test]
fn test_int_stream_reduce() {
    assert_eq!(
        run_bootstrap_int("Phase43Test.class", "testIntStreamReduce", "()I"),
        15,
    );
}

#[test]
fn test_int_stream_reduce_optional() {
    assert_eq!(
        run_bootstrap_int("Phase43Test.class", "testIntStreamReduceOptional", "()I"),
        14,
    );
}

#[test]
fn test_stream_reduce() {
    assert_eq!(
        run_bootstrap_int("Phase43Test.class", "testStreamReduce", "()I"),
        3,
    );
}

#[test]
fn test_stream_reduce_identity() {
    assert_eq!(
        run_bootstrap_int("Phase43Test.class", "testStreamReduceIdentity", "()I"),
        3,
    );
}

#[test]
fn test_arrays_sort_objects() {
    assert_eq!(
        run_bootstrap_int("Phase43Test.class", "testArraysSortObjects", "()I"),
        1,
    );
}

#[test]
fn test_string_value_of_char_array() {
    assert_eq!(
        run_bootstrap_int("Phase43Test.class", "testStringValueOfCharArray", "()I"),
        1,
    );
}

#[test]
fn test_new_string_from_char_array() {
    assert_eq!(
        run_bootstrap_int("Phase43Test.class", "testNewStringFromCharArray", "()I"),
        1,
    );
}

#[test]
fn test_collections_frequency_phase43() {
    assert_eq!(
        run_bootstrap_int(
            "Phase43Test.class",
            "testCollectionsFrequencyAlreadyDone",
            "()I"
        ),
        3,
    );
}

// ---- Phase 44 tests ----

#[test]
fn test_arrays_copy_of_range_int() {
    assert_eq!(
        run_bootstrap_int("Phase44Test.class", "testArraysCopyOfRangeInt", "()I"),
        9,
    );
}

#[test]
fn test_arrays_copy_of_range_object() {
    assert_eq!(
        run_bootstrap_int("Phase44Test.class", "testArraysCopyOfRangeObject", "()I"),
        1,
    );
}

#[test]
fn test_arrays_copy_of_range_pad() {
    assert_eq!(
        run_bootstrap_int("Phase44Test.class", "testArraysCopyOfRangePad", "()I"),
        5,
    );
}

#[test]
fn test_list_sub_list() {
    assert_eq!(
        run_bootstrap_int("Phase44Test.class", "testListSubList", "()I"),
        3,
    );
}

#[test]
fn test_list_sub_list_get() {
    assert_eq!(
        run_bootstrap_int("Phase44Test.class", "testListSubListGet", "()I"),
        1,
    );
}

#[test]
fn test_comparator_reversed() {
    assert_eq!(
        run_bootstrap_int("Phase44Test.class", "testComparatorReversed", "()I"),
        1,
    );
}

#[test]
fn test_string_intern() {
    assert_eq!(
        run_bootstrap_int("Phase44Test.class", "testStringIntern", "()I"),
        1,
    );
}

#[test]
fn test_collections_binary_search() {
    assert_eq!(
        run_bootstrap_int("Phase44Test.class", "testCollectionsBinarySearch", "()I"),
        2,
    );
}

#[test]
fn test_collections_binary_search_miss() {
    assert_eq!(
        run_bootstrap_int(
            "Phase44Test.class",
            "testCollectionsBinarySearchMiss",
            "()I"
        ),
        1,
    );
}

#[test]
fn test_remove_if() {
    assert_eq!(
        run_bootstrap_int("Phase44Test.class", "testRemoveIf", "()I"),
        3,
    );
}

#[test]
fn test_string_distinct_chars() {
    assert_eq!(
        run_bootstrap_int("Phase44Test.class", "testStringDistinctChars", "()I"),
        3,
    );
}

#[test]
fn test_map_values_stream() {
    assert_eq!(
        run_bootstrap_int("Phase44Test.class", "testMapValuesStream", "()I"),
        60,
    );
}

#[test]
fn test_map_entry_set_stream() {
    assert_eq!(
        run_bootstrap_int("Phase44Test.class", "testMapEntrySetStream", "()I"),
        6,
    );
}

// ---- Phase 45 ----

#[test]
fn test_list_for_each() {
    assert_eq!(
        run_bootstrap_int("Phase45Test.class", "testListForEach", "()I"),
        6,
    );
}

#[test]
fn test_stream_sorted_comparator() {
    assert_eq!(
        run_bootstrap_int("Phase45Test.class", "testStreamSortedComparator", "()I"),
        1,
    );
}

#[test]
fn test_stream_sorted_comparator_reversed() {
    assert_eq!(
        run_bootstrap_int(
            "Phase45Test.class",
            "testStreamSortedComparatorReversed",
            "()I"
        ),
        4,
    );
}

#[test]
fn test_arrays_to_string_int() {
    assert_eq!(
        run_bootstrap_int("Phase45Test.class", "testArraysToStringInt", "()I"),
        1,
    );
}

#[test]
fn test_arrays_to_string_object() {
    assert_eq!(
        run_bootstrap_int("Phase45Test.class", "testArraysToStringObject", "()I"),
        1,
    );
}

#[test]
fn test_map_replace() {
    assert_eq!(
        run_bootstrap_int("Phase45Test.class", "testMapReplace", "()I"),
        42,
    );
}

#[test]
fn test_map_replace_missing() {
    assert_eq!(
        run_bootstrap_int("Phase45Test.class", "testMapReplaceMissing", "()I"),
        1,
    );
}

#[test]
fn test_collections_swap() {
    assert_eq!(
        run_bootstrap_int("Phase45Test.class", "testCollectionsSwap", "()I"),
        1,
    );
}

#[test]
fn test_collectors_partitioning_by() {
    assert_eq!(
        run_bootstrap_int("Phase45Test.class", "testCollectorsPartitioningBy", "()I"),
        23,
    );
}

#[test]
fn test_int_stream_sorted() {
    assert_eq!(
        run_bootstrap_int("Phase45Test.class", "testIntStreamSorted", "()I"),
        6,
    );
}

#[test]
fn test_map_get_or_default() {
    assert_eq!(
        run_bootstrap_int("Phase45Test.class", "testMapGetOrDefault", "()I"),
        99,
    );
}

#[test]
fn test_string_format_newline() {
    assert_eq!(
        run_bootstrap_int("Phase45Test.class", "testStringFormatNewline", "()I"),
        1,
    );
}

#[test]
fn test_collections_unmodifiable_map() {
    assert_eq!(
        run_bootstrap_int("Phase45Test.class", "testCollectionsUnmodifiableMap", "()I"),
        7,
    );
}

// ---- Phase 46 ----

#[test]
fn test_string_repeat() {
    assert_eq!(
        run_bootstrap_int("Phase46Test.class", "testStringRepeat", "()I"),
        1,
    );
}

#[test]
fn test_string_is_blank() {
    assert_eq!(
        run_bootstrap_int("Phase46Test.class", "testStringIsBlank", "()I"),
        1,
    );
}

#[test]
fn test_string_is_blank_not() {
    assert_eq!(
        run_bootstrap_int("Phase46Test.class", "testStringIsBlankNot", "()I"),
        1,
    );
}

#[test]
fn test_string_lines() {
    assert_eq!(
        run_bootstrap_int("Phase46Test.class", "testStringLines", "()I"),
        3,
    );
}

#[test]
fn test_stream_take_while() {
    assert_eq!(
        run_bootstrap_int("Phase46Test.class", "testStreamTakeWhile", "()I"),
        3,
    );
}

#[test]
fn test_stream_drop_while() {
    assert_eq!(
        run_bootstrap_int("Phase46Test.class", "testStreamDropWhile", "()I"),
        2,
    );
}

#[test]
fn test_map_put_if_absent() {
    assert_eq!(
        run_bootstrap_int("Phase46Test.class", "testMapPutIfAbsent", "()I"),
        3,
    );
}

#[test]
fn test_map_compute_if_absent() {
    assert_eq!(
        run_bootstrap_int("Phase46Test.class", "testMapComputeIfAbsent", "()I"),
        42,
    );
}

#[test]
fn test_map_merge() {
    assert_eq!(
        run_bootstrap_int("Phase46Test.class", "testMapMerge", "()I"),
        15,
    );
}

#[test]
fn test_collections_reverse_order() {
    assert_eq!(
        run_bootstrap_int("Phase46Test.class", "testCollectionsReverseOrder", "()I"),
        3,
    );
}

#[test]
fn test_optional_of_nullable() {
    assert_eq!(
        run_bootstrap_int("Phase46Test.class", "testOptionalOfNullable", "()I"),
        1,
    );
}

#[test]
fn test_optional_of_nullable_null() {
    assert_eq!(
        run_bootstrap_int("Phase46Test.class", "testOptionalOfNullableNull", "()I"),
        0,
    );
}

// ---- Phase 47 ----

#[test]
fn test_list_of_size() {
    assert_eq!(
        run_bootstrap_int("Phase47Test.class", "testListOf", "()I"),
        3,
    );
}

#[test]
fn test_list_of_get() {
    assert_eq!(
        run_bootstrap_int("Phase47Test.class", "testListOfGet", "()I"),
        20,
    );
}

#[test]
fn test_set_of_size() {
    assert_eq!(
        run_bootstrap_int("Phase47Test.class", "testSetOf", "()I"),
        3,
    );
}

#[test]
fn test_set_of_contains() {
    assert_eq!(
        run_bootstrap_int("Phase47Test.class", "testSetOfContains", "()I"),
        1,
    );
}

#[test]
fn test_map_of_size() {
    assert_eq!(
        run_bootstrap_int("Phase47Test.class", "testMapOf", "()I"),
        2,
    );
}

#[test]
fn test_map_of_get() {
    assert_eq!(
        run_bootstrap_int("Phase47Test.class", "testMapOfGet", "()I"),
        42,
    );
}

#[test]
fn test_stream_flat_map_sum() {
    assert_eq!(
        run_bootstrap_int("Phase47Test.class", "testStreamFlatMap", "()I"),
        15,
    );
}

#[test]
fn test_stream_flat_map_size() {
    assert_eq!(
        run_bootstrap_int("Phase47Test.class", "testStreamFlatMapSize", "()I"),
        5,
    );
}

#[test]
fn test_string_builder_delete() {
    assert_eq!(
        run_bootstrap_int("Phase47Test.class", "testStringBuilderDelete", "()I"),
        1,
    );
}

#[test]
fn test_string_builder_insert() {
    assert_eq!(
        run_bootstrap_int("Phase47Test.class", "testStringBuilderInsert", "()I"),
        1,
    );
}

#[test]
fn test_string_builder_reverse() {
    assert_eq!(
        run_bootstrap_int("Phase47Test.class", "testStringBuilderReverse", "()I"),
        1,
    );
}

#[test]
fn test_collections_min() {
    assert_eq!(
        run_bootstrap_int("Phase47Test.class", "testCollectionsMin", "()I"),
        1,
    );
}

#[test]
fn test_collections_max() {
    assert_eq!(
        run_bootstrap_int("Phase47Test.class", "testCollectionsMax", "()I"),
        4,
    );
}

// ---- Phase 48 ----

#[test]
fn test_stream_generate() {
    assert_eq!(
        run_bootstrap_int("Phase48Test.class", "testStreamGenerate", "()I"),
        5,
    );
}

#[test]
fn test_stream_iterate() {
    assert_eq!(
        run_bootstrap_int("Phase48Test.class", "testStreamIterate", "()I"),
        10,
    );
}

#[test]
fn test_stream_concat() {
    assert_eq!(
        run_bootstrap_int("Phase48Test.class", "testStreamConcat", "()I"),
        5,
    );
}

#[test]
fn test_stream_empty() {
    assert_eq!(
        run_bootstrap_int("Phase48Test.class", "testStreamEmpty", "()I"),
        0,
    );
}

#[test]
fn test_int_stream_range_closed() {
    assert_eq!(
        run_bootstrap_int("Phase48Test.class", "testIntStreamRangeClosed", "()I"),
        15,
    );
}

#[test]
fn test_map_compute() {
    assert_eq!(
        run_bootstrap_int("Phase48Test.class", "testMapCompute", "()I"),
        20,
    );
}

#[test]
fn test_map_compute_absent() {
    assert_eq!(
        run_bootstrap_int("Phase48Test.class", "testMapComputeAbsent", "()I"),
        99,
    );
}

#[test]
fn test_optional_map() {
    assert_eq!(
        run_bootstrap_int("Phase48Test.class", "testOptionalMap", "()I"),
        5,
    );
}

#[test]
fn test_optional_filter() {
    assert_eq!(
        run_bootstrap_int("Phase48Test.class", "testOptionalFilter", "()I"),
        1,
    );
}

#[test]
fn test_optional_filter_empty() {
    assert_eq!(
        run_bootstrap_int("Phase48Test.class", "testOptionalFilterEmpty", "()I"),
        0,
    );
}

#[test]
fn test_optional_or_else() {
    assert_eq!(
        run_bootstrap_int("Phase48Test.class", "testOptionalOrElse", "()I"),
        99,
    );
}

#[test]
fn test_optional_or_else_get() {
    assert_eq!(
        run_bootstrap_int("Phase48Test.class", "testOptionalOrElseGet", "()I"),
        77,
    );
}

// ---- Phase 49 ----

#[test]
fn test_comparator_then_comparing() {
    assert_eq!(
        run_bootstrap_int("Phase49Test.class", "testComparatorThenComparing", "()I"),
        1,
    );
}

#[test]
fn test_predicate_and() {
    assert_eq!(
        run_bootstrap_int("Phase49Test.class", "testPredicateAnd", "()I"),
        1,
    );
}

#[test]
fn test_predicate_or() {
    assert_eq!(
        run_bootstrap_int("Phase49Test.class", "testPredicateOr", "()I"),
        2,
    );
}

#[test]
fn test_predicate_negate() {
    assert_eq!(
        run_bootstrap_int("Phase49Test.class", "testPredicateNegate", "()I"),
        3,
    );
}

#[test]
fn test_function_and_then() {
    assert_eq!(
        run_bootstrap_int("Phase49Test.class", "testFunctionAndThen", "()I"),
        10,
    );
}

#[test]
fn test_function_compose() {
    assert_eq!(
        run_bootstrap_int("Phase49Test.class", "testFunctionCompose", "()I"),
        14,
    );
}

#[test]
fn test_stream_map_to_long() {
    assert_eq!(
        run_bootstrap_int("Phase49Test.class", "testStreamMapToLong", "()I"),
        14,
    );
}

#[test]
fn test_stream_map_to_double() {
    assert_eq!(
        run_bootstrap_int("Phase49Test.class", "testStreamMapToDouble", "()I"),
        6,
    );
}

#[test]
fn test_collections_sort_comparator() {
    assert_eq!(
        run_bootstrap_int("Phase49Test.class", "testCollectionsSortComparator", "()I"),
        1,
    );
}

#[test]
fn test_integer_sum() {
    assert_eq!(
        run_bootstrap_int("Phase49Test.class", "testIntegerSum", "()I"),
        42,
    );
}

#[test]
fn test_integer_min_max() {
    assert_eq!(
        run_bootstrap_int("Phase49Test.class", "testIntegerMinMax", "()I"),
        8,
    );
}

// ---- Phase 50 ----

#[test]
fn test_stream_distinct() {
    assert_eq!(
        run_bootstrap_int("Phase50Test.class", "testStreamDistinct", "()I"),
        3
    );
}

#[test]
fn test_stream_distinct_collect() {
    assert_eq!(
        run_bootstrap_int("Phase50Test.class", "testStreamDistinctCollect", "()I"),
        3
    );
}

#[test]
fn test_stream_find_first() {
    assert_eq!(
        run_bootstrap_int("Phase50Test.class", "testStreamFindFirst", "()I"),
        6
    );
}

#[test]
fn test_stream_find_first_empty() {
    assert_eq!(
        run_bootstrap_int("Phase50Test.class", "testStreamFindFirstEmpty", "()I"),
        0
    );
}

#[test]
fn test_stream_all_match() {
    assert_eq!(
        run_bootstrap_int("Phase50Test.class", "testStreamAllMatch", "()I"),
        1
    );
}

#[test]
fn test_stream_all_match_fail() {
    assert_eq!(
        run_bootstrap_int("Phase50Test.class", "testStreamAllMatchFail", "()I"),
        0
    );
}

#[test]
fn test_stream_any_match() {
    assert_eq!(
        run_bootstrap_int("Phase50Test.class", "testStreamAnyMatch", "()I"),
        1
    );
}

#[test]
fn test_stream_any_match_fail() {
    assert_eq!(
        run_bootstrap_int("Phase50Test.class", "testStreamAnyMatchFail", "()I"),
        0
    );
}

#[test]
fn test_stream_none_match() {
    assert_eq!(
        run_bootstrap_int("Phase50Test.class", "testStreamNoneMatch", "()I"),
        1
    );
}

#[test]
fn test_stream_none_match_fail() {
    assert_eq!(
        run_bootstrap_int("Phase50Test.class", "testStreamNoneMatchFail", "()I"),
        0
    );
}

#[test]
fn test_collections_empty_list() {
    assert_eq!(
        run_bootstrap_int("Phase50Test.class", "testCollectionsEmptyList", "()I"),
        0
    );
}

#[test]
fn test_collections_empty_set() {
    assert_eq!(
        run_bootstrap_int("Phase50Test.class", "testCollectionsEmptySet", "()I"),
        0
    );
}

#[test]
fn test_collections_empty_map() {
    assert_eq!(
        run_bootstrap_int("Phase50Test.class", "testCollectionsEmptyMap", "()I"),
        0
    );
}

#[test]
fn test_collections_singleton_list() {
    assert_eq!(
        run_bootstrap_int("Phase50Test.class", "testCollectionsSingletonList", "()I"),
        1
    );
}

#[test]
fn test_objects_is_null() {
    assert_eq!(
        run_bootstrap_int("Phase50Test.class", "testObjectsIsNull", "()I"),
        1
    );
}

#[test]
fn test_objects_non_null() {
    assert_eq!(
        run_bootstrap_int("Phase50Test.class", "testObjectsNonNull", "()I"),
        1
    );
}

#[test]
fn test_objects_require_non_null() {
    assert_eq!(
        run_bootstrap_int("Phase50Test.class", "testObjectsRequireNonNull", "()I"),
        5
    );
}

// ---- Phase 51 ----

#[test]
fn test_long_stream_of() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testLongStreamOf", "()I"),
        15
    );
}

#[test]
fn test_long_stream_range() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testLongStreamRange", "()I"),
        5
    );
}

#[test]
fn test_long_stream_filter() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testLongStreamFilter", "()I"),
        6
    );
}

#[test]
fn test_long_stream_map() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testLongStreamMap", "()I"),
        14
    );
}

#[test]
fn test_long_stream_min() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testLongStreamMin", "()I"),
        1
    );
}

#[test]
fn test_long_stream_max() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testLongStreamMax", "()I"),
        5
    );
}

#[test]
fn test_long_stream_count() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testLongStreamCount", "()I"),
        3
    );
}

#[test]
fn test_long_stream_average() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testLongStreamAverage", "()I"),
        3
    );
}

#[test]
fn test_long_stream_to_array() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testLongStreamToArray", "()I"),
        9
    );
}

#[test]
fn test_long_stream_sorted() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testLongStreamSorted", "()I"),
        6
    );
}

#[test]
fn test_long_stream_boxed() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testLongStreamBoxed", "()I"),
        3
    );
}

#[test]
fn test_long_stream_reduce() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testLongStreamReduce", "()I"),
        15
    );
}

#[test]
fn test_double_stream_of() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testDoubleStreamOf", "()I"),
        6
    );
}

#[test]
fn test_double_stream_filter() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testDoubleStreamFilter", "()I"),
        3
    );
}

#[test]
fn test_double_stream_map() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testDoubleStreamMap", "()I"),
        6
    );
}

#[test]
fn test_double_stream_min() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testDoubleStreamMin", "()I"),
        1
    );
}

#[test]
fn test_double_stream_max() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testDoubleStreamMax", "()I"),
        4
    );
}

#[test]
fn test_double_stream_count() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testDoubleStreamCount", "()I"),
        3
    );
}

#[test]
fn test_double_stream_average() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testDoubleStreamAverage", "()I"),
        4
    );
}

#[test]
fn test_double_stream_to_array() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testDoubleStreamToArray", "()I"),
        6
    );
}

#[test]
fn test_int_stream_as_long_stream() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testIntStreamAsLongStream", "()I"),
        15
    );
}

#[test]
fn test_int_stream_as_double_stream() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testIntStreamAsDoubleStream", "()I"),
        6
    );
}

#[test]
fn test_collectors_summing_int() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testCollectorsSummingInt", "()I"),
        10
    );
}

#[test]
fn test_collectors_averaging_int() {
    assert_eq!(
        run_bootstrap_int("Phase51Test.class", "testCollectorsAveragingInt", "()I"),
        3
    );
}

// ===========================================================================
// Phase 52 — Stream.flatMap, Optional.flatMap/map/filter/orElse/ifPresent,
//            Arrays.stream, Collectors.joining(3-arg), Map.forEach, computeIfAbsent
// ===========================================================================

#[test]
fn test_stream_flat_map_p52() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testStreamFlatMap", "()I"),
        3
    );
}

#[test]
fn test_stream_flat_map_sum_p52() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testStreamFlatMapSum", "()I"),
        15
    );
}

#[test]
fn test_arrays_stream_int_p52() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testArraysStreamInt", "()I"),
        15
    );
}

#[test]
fn test_arrays_stream_range() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testArraysStreamRange", "()I"),
        50
    );
}

#[test]
fn test_optional_map_p52() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testOptionalMap", "()I"),
        5
    );
}

#[test]
fn test_optional_map_empty() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testOptionalMapEmpty", "()I"),
        0
    );
}

#[test]
fn test_optional_flat_map() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testOptionalFlatMap", "()I"),
        42
    );
}

#[test]
fn test_optional_or_else_p52() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testOptionalOrElse", "()I"),
        99
    );
}

#[test]
fn test_optional_or_else_present() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testOptionalOrElsePresent", "()I"),
        7
    );
}

#[test]
fn test_optional_or_else_get_p52() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testOptionalOrElseGet", "()I"),
        42
    );
}

#[test]
fn test_optional_if_present() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testOptionalIfPresent", "()I"),
        5
    );
}

#[test]
fn test_optional_if_present_empty() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testOptionalIfPresentEmpty", "()I"),
        0
    );
}

#[test]
fn test_optional_filter_p52() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testOptionalFilter", "()I"),
        10
    );
}

#[test]
fn test_optional_filter_out() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testOptionalFilterOut", "()I"),
        0
    );
}

#[test]
fn test_collectors_joining_p52() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testCollectorsJoining", "()I"),
        1
    );
}

#[test]
fn test_collectors_joining_full() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testCollectorsJoiningFull", "()I"),
        1
    );
}

#[test]
fn test_map_for_each_p52() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testMapForEach", "()I"),
        6
    );
}

#[test]
fn test_map_compute_if_absent_p52() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testMapComputeIfAbsent", "()I"),
        1
    );
}

#[test]
fn test_map_get_or_default_p52() {
    assert_eq!(
        run_bootstrap_int("Phase52Test.class", "testMapGetOrDefault", "()I"),
        4
    );
}

// ---- Phase 53 ----

#[test]
fn test_int_stream_find_first() {
    assert_eq!(
        run_bootstrap_int("Phase53Test.class", "testIntStreamFindFirst", "()I"),
        10
    );
}

#[test]
fn test_int_stream_find_first_empty() {
    assert_eq!(
        run_bootstrap_int("Phase53Test.class", "testIntStreamFindFirstEmpty", "()I"),
        0
    );
}

#[test]
fn test_int_stream_any_match() {
    assert_eq!(
        run_bootstrap_int("Phase53Test.class", "testIntStreamAnyMatch", "()I"),
        1
    );
}

#[test]
fn test_int_stream_any_match_fail() {
    assert_eq!(
        run_bootstrap_int("Phase53Test.class", "testIntStreamAnyMatchFail", "()I"),
        0
    );
}

#[test]
fn test_int_stream_all_match() {
    assert_eq!(
        run_bootstrap_int("Phase53Test.class", "testIntStreamAllMatch", "()I"),
        1
    );
}

#[test]
fn test_int_stream_all_match_fail() {
    assert_eq!(
        run_bootstrap_int("Phase53Test.class", "testIntStreamAllMatchFail", "()I"),
        0
    );
}

#[test]
fn test_int_stream_none_match() {
    assert_eq!(
        run_bootstrap_int("Phase53Test.class", "testIntStreamNoneMatch", "()I"),
        1
    );
}

#[test]
fn test_int_stream_none_match_fail() {
    assert_eq!(
        run_bootstrap_int("Phase53Test.class", "testIntStreamNoneMatchFail", "()I"),
        0
    );
}

#[test]
fn test_long_stream_find_first() {
    assert_eq!(
        run_bootstrap_int("Phase53Test.class", "testLongStreamFindFirst", "()I"),
        100
    );
}

#[test]
fn test_long_stream_any_match_p53() {
    assert_eq!(
        run_bootstrap_int("Phase53Test.class", "testLongStreamAnyMatch", "()I"),
        1
    );
}

#[test]
fn test_long_stream_all_match_p53() {
    assert_eq!(
        run_bootstrap_int("Phase53Test.class", "testLongStreamAllMatch", "()I"),
        1
    );
}

#[test]
fn test_long_stream_none_match_p53() {
    assert_eq!(
        run_bootstrap_int("Phase53Test.class", "testLongStreamNoneMatch", "()I"),
        1
    );
}

#[test]
fn test_int_stream_map_to_long() {
    assert_eq!(
        run_bootstrap_int("Phase53Test.class", "testIntStreamMapToLong", "()I"),
        15
    );
}

#[test]
fn test_comparator_comparing_long() {
    assert_eq!(
        run_bootstrap_int("Phase53Test.class", "testComparatorComparingLong", "()I"),
        2
    );
}

#[test]
fn test_optional_or() {
    assert_eq!(
        run_bootstrap_int("Phase53Test.class", "testOptionalOr", "()I"),
        8
    );
}

#[test]
fn test_optional_or_present() {
    assert_eq!(
        run_bootstrap_int("Phase53Test.class", "testOptionalOrPresent", "()I"),
        5
    );
}

#[test]
fn test_optional_if_present_or_else() {
    assert_eq!(
        run_bootstrap_int("Phase53Test.class", "testOptionalIfPresentOrElse", "()I"),
        5
    );
}

#[test]
fn test_optional_if_present_or_else_empty() {
    assert_eq!(
        run_bootstrap_int(
            "Phase53Test.class",
            "testOptionalIfPresentOrElseEmpty",
            "()I"
        ),
        99
    );
}

#[test]
fn test_collectors_to_unmodifiable_list() {
    assert_eq!(
        run_bootstrap_int(
            "Phase53Test.class",
            "testCollectorsToUnmodifiableList",
            "()I"
        ),
        3
    );
}

#[test]
fn test_collectors_to_unmodifiable_list_contents() {
    assert_eq!(
        run_bootstrap_int(
            "Phase53Test.class",
            "testCollectorsToUnmodifiableListContents",
            "()I"
        ),
        60
    );
}

#[test]
fn test_collectors_to_unmodifiable_set() {
    assert_eq!(
        run_bootstrap_int(
            "Phase53Test.class",
            "testCollectorsToUnmodifiableSet",
            "()I"
        ),
        3
    );
}

// ---- Phase 54 ----

#[test]
fn test_int_stream_limit() {
    assert_eq!(
        run_bootstrap_int("Phase54Test.class", "testIntStreamLimit", "()I"),
        6
    );
}

#[test]
fn test_int_stream_limit_zero() {
    assert_eq!(
        run_bootstrap_int("Phase54Test.class", "testIntStreamLimitZero", "()I"),
        0
    );
}

#[test]
fn test_int_stream_skip() {
    assert_eq!(
        run_bootstrap_int("Phase54Test.class", "testIntStreamSkip", "()I"),
        12
    );
}

#[test]
fn test_int_stream_skip_all() {
    assert_eq!(
        run_bootstrap_int("Phase54Test.class", "testIntStreamSkipAll", "()I"),
        0
    );
}

#[test]
fn test_int_stream_limit_skip() {
    assert_eq!(
        run_bootstrap_int("Phase54Test.class", "testIntStreamLimitSkip", "()I"),
        9
    );
}

#[test]
fn test_long_stream_limit() {
    assert_eq!(
        run_bootstrap_int("Phase54Test.class", "testLongStreamLimit", "()I"),
        60
    );
}

#[test]
fn test_long_stream_skip() {
    assert_eq!(
        run_bootstrap_int("Phase54Test.class", "testLongStreamSkip", "()I"),
        90
    );
}

#[test]
fn test_double_stream_limit() {
    assert_eq!(
        run_bootstrap_int("Phase54Test.class", "testDoubleStreamLimit", "()I"),
        6
    );
}

#[test]
fn test_double_stream_skip() {
    assert_eq!(
        run_bootstrap_int("Phase54Test.class", "testDoubleStreamSkip", "()I"),
        12
    );
}

#[test]
fn test_int_stream_flat_map() {
    assert_eq!(
        run_bootstrap_int("Phase54Test.class", "testIntStreamFlatMap", "()I"),
        66
    );
}

#[test]
fn test_int_stream_flat_map_range() {
    assert_eq!(
        run_bootstrap_int("Phase54Test.class", "testIntStreamFlatMapRange", "()I"),
        9
    );
}

#[test]
fn test_long_stream_flat_map() {
    assert_eq!(
        run_bootstrap_int("Phase54Test.class", "testLongStreamFlatMap", "()I"),
        66
    );
}

#[test]
fn test_collectors_mapping() {
    assert_eq!(
        run_bootstrap_int("Phase54Test.class", "testCollectorsMapping", "()I"),
        10
    );
}

#[test]
fn test_collectors_mapping_joining() {
    assert_eq!(
        run_bootstrap_int("Phase54Test.class", "testCollectorsMappingJoining", "()I"),
        1
    );
}

#[test]
fn test_collectors_mapping_grouped() {
    assert_eq!(
        run_bootstrap_int("Phase54Test.class", "testCollectorsMappingGrouped", "()I"),
        2
    );
}

// ---------------------------------------------------------------------------
// Phase 55: Complete DoubleStream, LongStream gaps, Collectors.summingLong/averagingDouble
// ---------------------------------------------------------------------------

#[test]
fn test_double_stream_for_each() {
    assert_eq!(
        run_bootstrap_int("Phase55Test.class", "testDoubleStreamForEach", "()I"),
        6
    );
}

#[test]
fn test_double_stream_any_match() {
    assert_eq!(
        run_bootstrap_int("Phase55Test.class", "testDoubleStreamAnyMatch", "()I"),
        1
    );
}

#[test]
fn test_double_stream_all_match() {
    assert_eq!(
        run_bootstrap_int("Phase55Test.class", "testDoubleStreamAllMatch", "()I"),
        1
    );
}

#[test]
fn test_double_stream_none_match() {
    assert_eq!(
        run_bootstrap_int("Phase55Test.class", "testDoubleStreamNoneMatch", "()I"),
        1
    );
}

#[test]
fn test_double_stream_find_first() {
    assert_eq!(
        run_bootstrap_int("Phase55Test.class", "testDoubleStreamFindFirst", "()I"),
        10
    );
}

#[test]
fn test_double_stream_find_first_empty() {
    assert_eq!(
        run_bootstrap_int("Phase55Test.class", "testDoubleStreamFindFirstEmpty", "()I"),
        0
    );
}

#[test]
fn test_double_stream_reduce_identity() {
    assert_eq!(
        run_bootstrap_int("Phase55Test.class", "testDoubleStreamReduceIdentity", "()I"),
        10
    );
}

#[test]
fn test_double_stream_reduce_optional() {
    assert_eq!(
        run_bootstrap_int("Phase55Test.class", "testDoubleStreamReduceOptional", "()I"),
        24
    );
}

#[test]
fn test_double_stream_flat_map() {
    assert_eq!(
        run_bootstrap_int("Phase55Test.class", "testDoubleStreamFlatMap", "()I"),
        66
    );
}

#[test]
fn test_double_stream_map_to_int() {
    assert_eq!(
        run_bootstrap_int("Phase55Test.class", "testDoubleStreamMapToInt", "()I"),
        6
    );
}

#[test]
fn test_double_stream_map_to_long() {
    assert_eq!(
        run_bootstrap_int("Phase55Test.class", "testDoubleStreamMapToLong", "()I"),
        600
    );
}

#[test]
fn test_double_stream_distinct() {
    assert_eq!(
        run_bootstrap_int("Phase55Test.class", "testDoubleStreamDistinct", "()I"),
        3
    );
}

#[test]
fn test_double_stream_boxed() {
    assert_eq!(
        run_bootstrap_int("Phase55Test.class", "testDoubleStreamBoxed", "()I"),
        3
    );
}

#[test]
fn test_long_stream_reduce_optional() {
    assert_eq!(
        run_bootstrap_int("Phase55Test.class", "testLongStreamReduceOptional", "()I"),
        24
    );
}

#[test]
fn test_long_stream_reduce_optional_empty() {
    assert_eq!(
        run_bootstrap_int(
            "Phase55Test.class",
            "testLongStreamReduceOptionalEmpty",
            "()I"
        ),
        0
    );
}

#[test]
fn test_long_stream_map_to_double() {
    assert_eq!(
        run_bootstrap_int("Phase55Test.class", "testLongStreamMapToDouble", "()I"),
        9
    );
}

#[test]
fn test_collectors_summing_long() {
    assert_eq!(
        run_bootstrap_int("Phase55Test.class", "testCollectorsSummingLong", "()I"),
        10
    );
}

#[test]
fn test_collectors_averaging_double() {
    assert_eq!(
        run_bootstrap_int("Phase55Test.class", "testCollectorsAveragingDouble", "()I"),
        3
    );
}

// ---------------------------------------------------------------------------
// Phase 56: Collectors.minBy/maxBy, summingDouble, averagingLong,
//           toUnmodifiableMap, collectingAndThen
// ---------------------------------------------------------------------------

#[test]
fn test_collectors_min_by() {
    assert_eq!(
        run_bootstrap_int("Phase56Test.class", "testCollectorsMinBy", "()I"),
        5
    );
}

#[test]
fn test_collectors_min_by_natural() {
    assert_eq!(
        run_bootstrap_int("Phase56Test.class", "testCollectorsMinByNatural", "()I"),
        1
    );
}

#[test]
fn test_collectors_max_by() {
    assert_eq!(
        run_bootstrap_int("Phase56Test.class", "testCollectorsMaxBy", "()I"),
        5
    );
}

#[test]
fn test_collectors_max_by_int() {
    assert_eq!(
        run_bootstrap_int("Phase56Test.class", "testCollectorsMaxByInt", "()I"),
        5
    );
}

#[test]
fn test_collectors_summing_double() {
    assert_eq!(
        run_bootstrap_int("Phase56Test.class", "testCollectorsSummingDouble", "()I"),
        10
    );
}

#[test]
fn test_collectors_averaging_long() {
    assert_eq!(
        run_bootstrap_int("Phase56Test.class", "testCollectorsAveragingLong", "()I"),
        3
    );
}

#[test]
fn test_collectors_to_unmodifiable_map() {
    assert_eq!(
        run_bootstrap_int(
            "Phase56Test.class",
            "testCollectorsToUnmodifiableMap",
            "()I"
        ),
        5
    );
}

#[test]
fn test_collectors_to_unmodifiable_map_size() {
    assert_eq!(
        run_bootstrap_int(
            "Phase56Test.class",
            "testCollectorsToUnmodifiableMapSize",
            "()I"
        ),
        3
    );
}

#[test]
fn test_collectors_collecting_and_then() {
    assert_eq!(
        run_bootstrap_int(
            "Phase56Test.class",
            "testCollectorsCollectingAndThen",
            "()I"
        ),
        4
    );
}

#[test]
fn test_collectors_collecting_and_then_join() {
    assert_eq!(
        run_bootstrap_int(
            "Phase56Test.class",
            "testCollectorsCollectingAndThenJoin",
            "()I"
        ),
        1
    );
}

#[test]
fn test_collectors_collecting_and_then_count() {
    assert_eq!(
        run_bootstrap_int(
            "Phase56Test.class",
            "testCollectorsCollectingAndThenCount",
            "()I"
        ),
        3
    );
}

// ---- Phase 57: Collectors.reducing ----

#[test]
fn test_collectors_reducing_no_identity() {
    assert_eq!(
        run_bootstrap_int(
            "Phase57Test.class",
            "testCollectorsReducingNoIdentity",
            "()I"
        ),
        10
    );
}

#[test]
fn test_collectors_reducing_no_identity_empty() {
    assert_eq!(
        run_bootstrap_int(
            "Phase57Test.class",
            "testCollectorsReducingNoIdentityEmpty",
            "()I"
        ),
        0
    );
}

#[test]
fn test_collectors_reducing_with_identity() {
    assert_eq!(
        run_bootstrap_int(
            "Phase57Test.class",
            "testCollectorsReducingWithIdentity",
            "()I"
        ),
        10
    );
}

#[test]
fn test_collectors_reducing_with_identity_empty() {
    assert_eq!(
        run_bootstrap_int(
            "Phase57Test.class",
            "testCollectorsReducingWithIdentityEmpty",
            "()I"
        ),
        42
    );
}

#[test]
fn test_collectors_reducing_mapping() {
    assert_eq!(
        run_bootstrap_int("Phase57Test.class", "testCollectorsReducingMapping", "()I"),
        10
    );
}

// ---- Phase 57: Stream.iterate predicate ----

#[test]
fn test_stream_iterate_predicate() {
    assert_eq!(
        run_bootstrap_int("Phase57Test.class", "testStreamIteratePredicate", "()I"),
        10
    );
}

#[test]
fn test_stream_iterate_predicate_empty() {
    assert_eq!(
        run_bootstrap_int(
            "Phase57Test.class",
            "testStreamIteratePredicateEmpty",
            "()I"
        ),
        0
    );
}

#[test]
fn test_stream_iterate_predicate_count() {
    assert_eq!(
        run_bootstrap_int(
            "Phase57Test.class",
            "testStreamIteratePredicateCount",
            "()I"
        ),
        5
    );
}

// ---- Phase 57: Optional.stream() ----

#[test]
fn test_optional_stream_present() {
    assert_eq!(
        run_bootstrap_int("Phase57Test.class", "testOptionalStreamPresent", "()I"),
        1
    );
}

#[test]
fn test_optional_stream_empty() {
    assert_eq!(
        run_bootstrap_int("Phase57Test.class", "testOptionalStreamEmpty", "()I"),
        0
    );
}

// ---- Phase 57: ArrayDeque completion ----

#[test]
fn test_arraydeque_add_first_peek_first() {
    assert_eq!(
        run_bootstrap_int(
            "Phase57Test.class",
            "testArrayDequeAddFirstPeekFirst",
            "()I"
        ),
        2
    );
}

#[test]
fn test_arraydeque_add_last_peek_last() {
    assert_eq!(
        run_bootstrap_int("Phase57Test.class", "testArrayDequeAddLastPeekLast", "()I"),
        2
    );
}

#[test]
fn test_arraydeque_poll_first() {
    assert_eq!(
        run_bootstrap_int("Phase57Test.class", "testArrayDequePollFirst", "()I"),
        11
    );
}

#[test]
fn test_arraydeque_poll_last() {
    assert_eq!(
        run_bootstrap_int("Phase57Test.class", "testArrayDequePollLast", "()I"),
        21
    );
}

#[test]
fn test_arraydeque_contains() {
    assert_eq!(
        run_bootstrap_int("Phase57Test.class", "testArrayDequeContains", "()I"),
        2
    );
}

#[test]
fn test_arraydeque_clear() {
    assert_eq!(
        run_bootstrap_int("Phase57Test.class", "testArrayDequeClear", "()I"),
        0
    );
}

#[test]
fn test_arraydeque_for_each() {
    assert_eq!(
        run_bootstrap_int("Phase57Test.class", "testArrayDequeForEach", "()I"),
        6
    );
}

// ---- Phase 58: Comparator.comparingDouble ----

#[test]
fn test_comparator_comparing_double() {
    assert_eq!(
        run_bootstrap_int("Phase58Test.class", "testComparatorComparingDouble", "()I"),
        2
    );
}

#[test]
fn test_comparator_comparing_double_reversed() {
    assert_eq!(
        run_bootstrap_int(
            "Phase58Test.class",
            "testComparatorComparingDoubleReversed",
            "()I"
        ),
        5
    );
}

// ---- Phase 58: Map.copyOf ----

#[test]
fn test_map_copy_of() {
    assert_eq!(
        run_bootstrap_int("Phase58Test.class", "testMapCopyOf", "()I"),
        3
    );
}

#[test]
fn test_map_copy_of_contents() {
    assert_eq!(
        run_bootstrap_int("Phase58Test.class", "testMapCopyOfContents", "()I"),
        5
    );
}

// ---- Phase 58: Map.entry ----

#[test]
fn test_map_entry() {
    assert_eq!(
        run_bootstrap_int("Phase58Test.class", "testMapEntry", "()I"),
        45
    );
}

// ---- Phase 58: Map.ofEntries ----

#[test]
fn test_map_of_entries() {
    assert_eq!(
        run_bootstrap_int("Phase58Test.class", "testMapOfEntries", "()I"),
        3
    );
}

#[test]
fn test_map_of_entries_get() {
    assert_eq!(
        run_bootstrap_int("Phase58Test.class", "testMapOfEntriesGet", "()I"),
        20
    );
}

// ---- Phase 58: Collections.singletonMap ----

#[test]
fn test_collections_singleton_map() {
    assert_eq!(
        run_bootstrap_int("Phase58Test.class", "testCollectionsSingletonMap", "()I"),
        100
    );
}

// ---- Phase 58: Collections.singleton (set) ----

#[test]
fn test_collections_singleton_set() {
    assert_eq!(
        run_bootstrap_int("Phase58Test.class", "testCollectionsSingletonSet", "()I"),
        2
    );
}

// ---- Phase 58: Collections.unmodifiableSet ----

#[test]
fn test_collections_unmodifiable_set() {
    assert_eq!(
        run_bootstrap_int("Phase58Test.class", "testCollectionsUnmodifiableSet", "()I"),
        3
    );
}

#[test]
fn test_collections_unmodifiable_set_contains() {
    assert_eq!(
        run_bootstrap_int(
            "Phase58Test.class",
            "testCollectionsUnmodifiableSetContains",
            "()I"
        ),
        1
    );
}

// ---- Phase 59: Stream.flatMapToInt/Long/Double, Collectors.toMap (3-arg), collection forEach ----

#[test]
fn test_stream_flat_map_to_int() {
    assert_eq!(
        run_bootstrap_int("Phase59Test.class", "testStreamFlatMapToInt", "()I"),
        394
    );
}

#[test]
fn test_stream_flat_map_to_int_count() {
    assert_eq!(
        run_bootstrap_int("Phase59Test.class", "testStreamFlatMapToIntCount", "()I"),
        10
    );
}

#[test]
fn test_stream_flat_map_to_long() {
    assert_eq!(
        run_bootstrap_int("Phase59Test.class", "testStreamFlatMapToLong", "()I"),
        66
    );
}

#[test]
fn test_stream_flat_map_to_double() {
    assert_eq!(
        run_bootstrap_int("Phase59Test.class", "testStreamFlatMapToDouble", "()I"),
        6
    );
}

#[test]
fn test_collectors_to_map_merge() {
    assert_eq!(
        run_bootstrap_int("Phase59Test.class", "testCollectorsToMapMerge", "()I"),
        2
    );
}

#[test]
fn test_collectors_to_map_merge_size() {
    assert_eq!(
        run_bootstrap_int("Phase59Test.class", "testCollectorsToMapMergeSize", "()I"),
        3
    );
}

#[test]
fn test_collectors_to_map_merge_value() {
    assert_eq!(
        run_bootstrap_int("Phase59Test.class", "testCollectorsToMapMergeValue", "()I"),
        5
    );
}

#[test]
fn test_hashset_for_each() {
    assert_eq!(
        run_bootstrap_int("Phase59Test.class", "testHashSetForEach", "()I"),
        6
    );
}

#[test]
fn test_treeset_for_each() {
    assert_eq!(
        run_bootstrap_int("Phase59Test.class", "testTreeSetForEach", "()I"),
        3
    );
}

#[test]
fn test_treemap_for_each() {
    assert_eq!(
        run_bootstrap_int("Phase59Test.class", "testTreeMapForEach", "()I"),
        6
    );
}

#[test]
fn test_linked_list_for_each() {
    assert_eq!(
        run_bootstrap_int("Phase59Test.class", "testLinkedListForEach", "()I"),
        60
    );
}

#[test]
fn test_linkedhashmap_for_each() {
    assert_eq!(
        run_bootstrap_int("Phase59Test.class", "testLinkedHashMapForEach", "()I"),
        60
    );
}

#[test]
fn test_priorityqueue_for_each() {
    assert_eq!(
        run_bootstrap_int("Phase59Test.class", "testPriorityQueueForEach", "()I"),
        15
    );
}

// ---- Phase 60: takeWhile/dropWhile, Integer/Long/Double compare/max/min, TreeMap/Set ----

#[test]
fn test_int_stream_take_while() {
    assert_eq!(
        run_bootstrap_int("Phase60Test.class", "testIntStreamTakeWhile", "()I"),
        6
    );
}

#[test]
fn test_int_stream_drop_while() {
    assert_eq!(
        run_bootstrap_int("Phase60Test.class", "testIntStreamDropWhile", "()I"),
        9
    );
}

#[test]
fn test_long_stream_take_while() {
    assert_eq!(
        run_bootstrap_int("Phase60Test.class", "testLongStreamTakeWhile", "()I"),
        30
    );
}

#[test]
fn test_long_stream_drop_while() {
    assert_eq!(
        run_bootstrap_int("Phase60Test.class", "testLongStreamDropWhile", "()I"),
        70
    );
}

#[test]
fn test_double_stream_take_while() {
    assert_eq!(
        run_bootstrap_int("Phase60Test.class", "testDoubleStreamTakeWhile", "()I"),
        4
    );
}

#[test]
fn test_double_stream_drop_while() {
    assert_eq!(
        run_bootstrap_int("Phase60Test.class", "testDoubleStreamDropWhile", "()I"),
        8
    );
}

#[test]
fn test_p60_integer_compare() {
    let v = run_bootstrap_int("Phase60Test.class", "testIntegerCompare", "()I");
    assert!(v > 0, "expected positive, got {v}");
}

#[test]
fn test_p60_integer_compare_equal() {
    assert_eq!(
        run_bootstrap_int("Phase60Test.class", "testIntegerCompareEqual", "()I"),
        0
    );
}

#[test]
fn test_p60_integer_max() {
    assert_eq!(
        run_bootstrap_int("Phase60Test.class", "testIntegerMax", "()I"),
        20
    );
}

#[test]
fn test_p60_integer_min() {
    assert_eq!(
        run_bootstrap_int("Phase60Test.class", "testIntegerMin", "()I"),
        10
    );
}

#[test]
fn test_p60_long_compare() {
    let v = run_bootstrap_int("Phase60Test.class", "testLongCompare", "()I");
    assert!(v > 0, "expected positive, got {v}");
}

#[test]
fn test_p60_long_max() {
    assert_eq!(
        run_bootstrap_int("Phase60Test.class", "testLongMax", "()I"),
        200
    );
}

#[test]
fn test_p60_long_min() {
    assert_eq!(
        run_bootstrap_int("Phase60Test.class", "testLongMin", "()I"),
        100
    );
}

#[test]
fn test_p60_double_compare() {
    let v = run_bootstrap_int("Phase60Test.class", "testDoubleCompare", "()I");
    assert!(v > 0, "expected positive, got {v}");
}

#[test]
fn test_p60_double_max() {
    assert_eq!(
        run_bootstrap_int("Phase60Test.class", "testDoubleMax", "()I"),
        2
    );
}

#[test]
fn test_p60_double_min() {
    assert_eq!(
        run_bootstrap_int("Phase60Test.class", "testDoubleMin", "()I"),
        1
    );
}

#[test]
fn test_treemap_key_set() {
    assert_eq!(
        run_bootstrap_int("Phase60Test.class", "testTreeMapKeySet", "()I"),
        3
    );
}

#[test]
fn test_treemap_values() {
    assert_eq!(
        run_bootstrap_int("Phase60Test.class", "testTreeMapValues", "()I"),
        30
    );
}

#[test]
fn test_treemap_get_or_default() {
    assert_eq!(
        run_bootstrap_int("Phase60Test.class", "testTreeMapGetOrDefault", "()I"),
        141
    );
}

#[test]
fn test_treeset_stream() {
    assert_eq!(
        run_bootstrap_int("Phase60Test.class", "testTreeSetStream", "()I"),
        3
    );
}

// ---- java.time.ZoneId (minimal-for-boot) ----

#[test]
fn test_zoneid_system_default_is_utc() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let zone =
        native_zoneid_system_default(&[], &mut heap, &mut sink, &mut NativeControl::default())
            .expect("systemDefault should succeed")
            .expect("systemDefault should return a ZoneId");
    let Slot::Reference(Some(zone_ref)) = zone else {
        panic!("expected ZoneId reference");
    };
    let id = native_zoneid_get_id(
        &[Slot::Reference(Some(zone_ref))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .expect("getId should succeed")
    .expect("getId should return a String");
    let Slot::Reference(Some(id_ref)) = id else {
        panic!("expected String reference");
    };
    assert_eq!(
        heap.get(id_ref).unwrap().string_value.as_deref(),
        Some("UTC")
    );
}

#[test]
fn test_zoneid_of_round_trips_id() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let name_ref = heap.allocate_string("America/New_York".to_string());
    let zone = native_zoneid_of(
        &[Slot::Reference(Some(name_ref))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .expect("ZoneId.of should succeed")
    .expect("ZoneId.of should return a ZoneId");
    let Slot::Reference(Some(zone_ref)) = zone else {
        panic!("expected ZoneId reference");
    };
    let id = native_zoneid_get_id(
        &[Slot::Reference(Some(zone_ref))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .expect("getId should succeed")
    .expect("getId should return a String");
    let Slot::Reference(Some(id_ref)) = id else {
        panic!("expected String reference");
    };
    assert_eq!(
        heap.get(id_ref).unwrap().string_value.as_deref(),
        Some("America/New_York")
    );
}

// ---- java.util.Locale (minimal-for-boot) ----

#[test]
fn test_locale_get_default_is_en_us() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let locale =
        native_locale_get_default(&[], &mut heap, &mut sink, &mut NativeControl::default())
            .expect("getDefault should succeed")
            .expect("getDefault should return a Locale");
    let Slot::Reference(Some(locale_ref)) = locale else {
        panic!("expected Locale reference");
    };
    // The documented fixed default: language "en", country "US".
    let language = native_locale_get_language(
        &[Slot::Reference(Some(locale_ref))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .expect("getLanguage should succeed")
    .expect("getLanguage should return a String");
    let country = native_locale_get_country(
        &[Slot::Reference(Some(locale_ref))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .expect("getCountry should succeed")
    .expect("getCountry should return a String");
    let to_string = native_locale_to_string(
        &[Slot::Reference(Some(locale_ref))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .expect("toString should succeed")
    .expect("toString should return a String");
    let (
        Slot::Reference(Some(lang_ref)),
        Slot::Reference(Some(country_ref)),
        Slot::Reference(Some(str_ref)),
    ) = (language, country, to_string)
    else {
        panic!("expected String references");
    };
    assert_eq!(
        heap.get(lang_ref).unwrap().string_value.as_deref(),
        Some("en")
    );
    assert_eq!(
        heap.get(country_ref).unwrap().string_value.as_deref(),
        Some("US")
    );
    assert_eq!(
        heap.get(str_ref).unwrap().string_value.as_deref(),
        Some("en_US")
    );
}

// ---- Phase 62: java.time (LocalDate, Duration, Period, Instant) ----

#[test]
fn test_localdate_components() {
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testLocalDateComponents", "()I"),
        20_240_315
    );
}

#[test]
fn test_localdate_plus_days() {
    // 2024-01-01 + 31 days = 2024-02-01
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testLocalDatePlusDays", "()I"),
        20_240_201
    );
}

#[test]
fn test_localdate_minus_days() {
    // 2024-03-01 - 1 day = 2024-02-29 (leap year)
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testLocalDateMinusDays", "()I"),
        20_240_229
    );
}

#[test]
fn test_localdate_plus_months() {
    // 2023-11-30 + 3 months = 2024-02-29 (clamped to Feb end in leap year)
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testLocalDatePlusMonths", "()I"),
        20_240_229
    );
}

#[test]
fn test_localdate_plus_years() {
    // 2020-06-15 + 4 years = 2024-06-15
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testLocalDatePlusYears", "()I"),
        20_240_615
    );
}

#[test]
fn test_localdate_comparisons() {
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testLocalDateComparisons", "()I"),
        7
    );
}

#[test]
fn test_localdate_epoch_day() {
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testLocalDateEpochDay", "()I"),
        0
    );
}

#[test]
fn test_duration_seconds() {
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testDurationSeconds", "()I"),
        3723
    );
}

#[test]
fn test_duration_minutes() {
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testDurationMinutes", "()I"),
        2
    );
}

#[test]
fn test_duration_arithmetic() {
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testDurationArithmetic", "()I"),
        6
    );
}

#[test]
fn test_duration_flags() {
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testDurationFlags", "()I"),
        3
    );
}

#[test]
fn test_period_components() {
    // Period.of(1, 6, 15): 1*10000 + 6*100 + 15 = 10615
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testPeriodComponents", "()I"),
        10615
    );
}

#[test]
fn test_period_factories() {
    // 7 + 300 + 20000 = 20307
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testPeriodFactories", "()I"),
        20307
    );
}

#[test]
fn test_period_flags() {
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testPeriodFlags", "()I"),
        7
    );
}

#[test]
fn test_instant_epoch_second() {
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testInstantEpochSecond", "()I"),
        1_000_000
    );
}

#[test]
fn test_instant_epoch_milli() {
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testInstantEpochMilli", "()I"),
        5000
    );
}

#[test]
fn test_instant_comparisons() {
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testInstantComparisons", "()I"),
        3
    );
}

#[test]
fn test_localdate_now() {
    // interpreter epoch 0 = 1970-01-01, so getYear() = 1970
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testLocalDateNow", "()I"),
        1970
    );
}

#[test]
fn test_localdate_to_string() {
    // "2024-12-31" has length 10
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testLocalDateToString", "()I"),
        10
    );
}

#[test]
fn test_duration_to_seconds() {
    // 2 minutes = 120 seconds
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testDurationToSeconds", "()I"),
        120
    );
}

#[test]
fn test_dispatch_cache_hot_path() {
    // exercises invokestatic dispatch cache warm path (5 calls to same method)
    assert_eq!(
        run_bootstrap_int("Phase62Test.class", "testDispatchCacheHotPath", "()I"),
        10
    );
}

// ---- Phase 63: LocalDateTime ----

#[test]
fn test_localdatetime_components() {
    // 2024-06-15 → 20_240_615
    assert_eq!(
        run_bootstrap_int("Phase63Test.class", "testLocalDateTimeComponents", "()I"),
        20_240_615
    );
}

#[test]
fn test_localdatetime_time() {
    // 23:45:59 → 23*10000 + 45*100 + 59 = 234559
    assert_eq!(
        run_bootstrap_int("Phase63Test.class", "testLocalDateTimeTime", "()I"),
        234_559
    );
}

#[test]
fn test_localdatetime_to_local_date() {
    // 2023-12-25 → 20_231_225
    assert_eq!(
        run_bootstrap_int("Phase63Test.class", "testLocalDateTimeToLocalDate", "()I"),
        20_231_225
    );
}

#[test]
fn test_localdatetime_ordering() {
    assert_eq!(
        run_bootstrap_int("Phase63Test.class", "testLocalDateTimeOrdering", "()I"),
        15
    );
}

#[test]
fn test_localdatetime_to_string() {
    // "2024-03-15T09:05:07" length = 19
    assert_eq!(
        run_bootstrap_int("Phase63Test.class", "testLocalDateTimeToString", "()I"),
        19
    );
}

#[test]
fn test_localdatetime_plus_days() {
    // 2024-01-30 + 2 days = 2024-02-01
    assert_eq!(
        run_bootstrap_int("Phase63Test.class", "testLocalDateTimePlusDays", "()I"),
        20_240_201
    );
}

#[test]
fn test_localdatetime_with_hour() {
    assert_eq!(
        run_bootstrap_int("Phase63Test.class", "testLocalDateTimeWithHour", "()I"),
        18
    );
}

#[test]
fn test_localdatetime_now() {
    assert_eq!(
        run_bootstrap_int("Phase63Test.class", "testLocalDateTimeNow", "()I"),
        1970
    );
}

macro_rules! java_time_basic_int_test {
    ($name:ident, $method:literal) => {
        #[test]
        fn $name() {
            assert_eq!(
                run_bootstrap_int_completion("JavaTimeBasicTest.class", $method, "()I"),
                1
            );
        }
    };
}

// ---- Issue 658: richer java.time surface ----
java_time_basic_int_test!(
    java_time_instant_epoch_second_and_nano,
    "instantEpochSecondAndNano"
);
java_time_basic_int_test!(
    java_time_instant_arithmetic_is_immutable,
    "instantArithmeticIsImmutable"
);
java_time_basic_int_test!(java_time_instant_parse_round_trip, "instantParseRoundTrip");
java_time_basic_int_test!(
    java_time_instant_now_within_system_millis_window,
    "instantNowWithinSystemMillisWindow"
);
java_time_basic_int_test!(
    java_time_instant_compare_equals_hash_code,
    "instantCompareEqualsHashCode"
);
java_time_basic_int_test!(
    java_time_duration_factories_and_accessors,
    "durationFactoriesAndAccessors"
);
java_time_basic_int_test!(
    java_time_duration_between_instants,
    "durationBetweenInstants"
);
java_time_basic_int_test!(
    java_time_duration_arithmetic_is_immutable,
    "durationArithmeticIsImmutable"
);
java_time_basic_int_test!(
    java_time_duration_compare_equals_hash_code,
    "durationCompareEqualsHashCode"
);
java_time_basic_int_test!(
    java_time_local_date_arithmetic_is_immutable,
    "localDateArithmeticIsImmutable"
);
java_time_basic_int_test!(
    java_time_local_date_parse_round_trip,
    "localDateParseRoundTrip"
);
java_time_basic_int_test!(
    java_time_local_date_compare_equals_hash_code,
    "localDateCompareEqualsHashCode"
);
java_time_basic_int_test!(
    java_time_local_date_time_arithmetic_is_immutable,
    "localDateTimeArithmeticIsImmutable"
);
java_time_basic_int_test!(
    java_time_local_date_time_parse_round_trip,
    "localDateTimeParseRoundTrip"
);
java_time_basic_int_test!(
    java_time_local_date_time_compare_equals_hash_code,
    "localDateTimeCompareEqualsHashCode"
);
java_time_basic_int_test!(
    java_time_formatter_statics_resolve,
    "dateTimeFormatterStaticsResolve"
);
java_time_basic_int_test!(
    java_time_malformed_parse_raises_date_time_parse_exception,
    "malformedParseRaisesDateTimeParseException"
);

// Phase 64: String.indent, StringBuilder.setCharAt, Collections.disjoint, HashMap.computeIfPresent
#[test]
fn test_string_indent_positive() {
    assert_eq!(
        run_bootstrap_int("Phase64Test.class", "testStringIndentPositive", "()I"),
        20
    );
}

#[test]
fn test_string_indent_negative() {
    assert_eq!(
        run_bootstrap_int("Phase64Test.class", "testStringIndentNegative", "()I"),
        16
    );
}

#[test]
fn test_string_indent_zero() {
    assert_eq!(
        run_bootstrap_int("Phase64Test.class", "testStringIndentZero", "()I"),
        4
    );
}

#[test]
fn test_string_indent_starts_with() {
    assert_eq!(
        run_bootstrap_int("Phase64Test.class", "testStringIndentStartsWith", "()I"),
        15
    );
}

#[test]
fn test_stringbuilder_set_char_at() {
    assert_eq!(
        run_bootstrap_int("Phase64Test.class", "testStringBuilderSetCharAt", "()I"),
        5
    );
}

#[test]
fn test_stringbuilder_set_char_at_value() {
    assert_eq!(
        run_bootstrap_int(
            "Phase64Test.class",
            "testStringBuilderSetCharAtValue",
            "()I"
        ),
        7
    );
}

#[test]
fn test_collections_disjoint_true() {
    assert_eq!(
        run_bootstrap_int("Phase64Test.class", "testCollectionsDisjointTrue", "()I"),
        1
    );
}

#[test]
fn test_collections_disjoint_false() {
    assert_eq!(
        run_bootstrap_int("Phase64Test.class", "testCollectionsDisjointFalse", "()I"),
        0
    );
}

#[test]
fn test_hashmap_compute_if_present_hit() {
    assert_eq!(
        run_bootstrap_int("Phase64Test.class", "testHashMapComputeIfPresentHit", "()I"),
        15
    );
}

#[test]
fn test_hashmap_compute_if_present_miss() {
    assert_eq!(
        run_bootstrap_int(
            "Phase64Test.class",
            "testHashMapComputeIfPresentMiss",
            "()I"
        ),
        1
    );
}

// Phase 65: Java 11 String methods, Integer radix conversions, HashMap callbacks
#[test]
fn test_p65_string_strip() {
    assert_eq!(
        run_bootstrap_int("Phase65Test.class", "testStringStrip", "()I"),
        11
    );
}

#[test]
fn test_p65_string_strip_leading() {
    assert_eq!(
        run_bootstrap_int("Phase65Test.class", "testStringStripLeading", "()I"),
        3
    );
}

#[test]
fn test_p65_string_strip_trailing() {
    assert_eq!(
        run_bootstrap_int("Phase65Test.class", "testStringStripTrailing", "()I"),
        3
    );
}

#[test]
fn test_p65_string_repeat() {
    assert_eq!(
        run_bootstrap_int("Phase65Test.class", "testStringRepeat", "()I"),
        8
    );
}

#[test]
fn test_p65_string_repeat_zero() {
    assert_eq!(
        run_bootstrap_int("Phase65Test.class", "testStringRepeatZero", "()I"),
        0
    );
}

#[test]
fn test_p65_string_is_blank_true() {
    assert_eq!(
        run_bootstrap_int("Phase65Test.class", "testStringIsBlankTrue", "()I"),
        1
    );
}

#[test]
fn test_p65_string_is_blank_false() {
    assert_eq!(
        run_bootstrap_int("Phase65Test.class", "testStringIsBlankFalse", "()I"),
        0
    );
}

#[test]
fn test_p65_integer_to_binary_string() {
    assert_eq!(
        run_bootstrap_int("Phase65Test.class", "testIntegerToBinaryString", "()I"),
        4
    );
}

#[test]
fn test_p65_integer_to_hex_string() {
    assert_eq!(
        run_bootstrap_int("Phase65Test.class", "testIntegerToHexString", "()I"),
        2
    );
}

#[test]
fn test_p65_integer_to_octal_string() {
    assert_eq!(
        run_bootstrap_int("Phase65Test.class", "testIntegerToOctalString", "()I"),
        2
    );
}

#[test]
fn test_p65_integer_to_hex_string_value() {
    assert_eq!(
        run_bootstrap_int("Phase65Test.class", "testIntegerToHexStringValue", "()I"),
        15
    );
}

#[test]
fn test_p65_hashmap_compute_if_absent_miss() {
    assert_eq!(
        run_bootstrap_int("Phase65Test.class", "testHashMapComputeIfAbsentMiss", "()I"),
        1
    );
}

#[test]
fn test_p65_hashmap_compute_if_absent_hit() {
    assert_eq!(
        run_bootstrap_int("Phase65Test.class", "testHashMapComputeIfAbsentHit", "()I"),
        42
    );
}

#[test]
fn test_p65_hashmap_for_each() {
    assert_eq!(
        run_bootstrap_int("Phase65Test.class", "testHashMapForEach", "()I"),
        60
    );
}

// Phase 66: HashMap.replaceAll, Iterator.remove
#[test]
fn test_p66_hashmap_replace_all() {
    assert_eq!(
        run_bootstrap_int("Phase66Test.class", "testHashMapReplaceAll", "()I"),
        60
    );
}

#[test]
fn test_p66_hashmap_replace_all_length() {
    assert_eq!(
        run_bootstrap_int("Phase66Test.class", "testHashMapReplaceAllLength", "()I"),
        10
    );
}

#[test]
fn test_p66_iterator_remove() {
    assert_eq!(
        run_bootstrap_int("Phase66Test.class", "testIteratorRemove", "()I"),
        3
    );
}

#[test]
fn test_p66_iterator_remove_all() {
    assert_eq!(
        run_bootstrap_int("Phase66Test.class", "testIteratorRemoveAll", "()I"),
        0
    );
}

#[test]
fn test_p66_iterator_remove_sum() {
    assert_eq!(
        run_bootstrap_int("Phase66Test.class", "testIteratorRemoveSum", "()I"),
        12
    );
}

// Phase 67: method-ref unboxing (adapt_args_for_impl_desc), and modern Java syntax probes
#[test]
fn test_p67_stream_reduce_method_ref() {
    // Integer::sum as BinaryOperator<Integer> — requires boxed→unboxed arg adaptation
    assert_eq!(
        run_bootstrap_int("ProbeRun.class", "testStreamReduce", "()I"),
        15
    );
}

#[test]
fn test_p67_record() {
    assert_eq!(run_bootstrap_int("ProbeRun.class", "testRecord", "()I"), 7);
}

#[test]
fn test_p67_pattern_match() {
    assert_eq!(
        run_bootstrap_int("ProbeRun.class", "testPatternMatch", "()I"),
        5
    );
}

#[test]
fn test_p67_text_block() {
    assert_eq!(
        run_bootstrap_int("ProbeRun.class", "testTextBlock", "()I"),
        11
    );
}

#[test]
fn test_p67_switch_expr() {
    assert_eq!(
        run_bootstrap_int("ProbeRun.class", "testSwitchExpr", "()I"),
        3
    );
}

// Phase 68: stream operations — mapToInt method refs, groupingBy, distinct, sorted, limit/skip
#[test]
fn test_p68_map_to_int_method_ref() {
    assert_eq!(
        run_bootstrap_int("Phase68Test.class", "testMapToIntMethodRef", "()I"),
        10
    );
}

#[test]
fn test_p68_map_to_int_lambda() {
    assert_eq!(
        run_bootstrap_int("Phase68Test.class", "testMapToIntLambda", "()I"),
        14
    );
}

#[test]
fn test_p68_collectors_summing_int() {
    assert_eq!(
        run_bootstrap_int("Phase68Test.class", "testCollectorsSummingInt", "()I"),
        9
    );
}

#[test]
fn test_p68_map_to_long() {
    assert_eq!(
        run_bootstrap_int("Phase68Test.class", "testMapToLong", "()I"),
        12
    );
}

#[test]
fn test_p68_grouping_by() {
    assert_eq!(
        run_bootstrap_int("Phase68Test.class", "testGroupingBy", "()I"),
        4
    );
}

#[test]
fn test_p68_stream_distinct() {
    assert_eq!(
        run_bootstrap_int("Phase68Test.class", "testStreamDistinct", "()I"),
        3
    );
}

#[test]
fn test_p68_stream_sorted() {
    assert_eq!(
        run_bootstrap_int("Phase68Test.class", "testStreamSorted", "()I"),
        15
    );
}

#[test]
fn test_p68_stream_limit_skip() {
    assert_eq!(
        run_bootstrap_int("Phase68Test.class", "testStreamLimitSkip", "()I"),
        12
    );
}

// Phase 69: Collectors.toMap, Stream.flatMap/peek, groupingBy+counting, OptionalInt.orElse, String.chars()
#[test]
fn test_p69_collectors_to_map() {
    assert_eq!(
        run_bootstrap_int("Phase69Test.class", "testCollectorsToMap", "()I"),
        17
    );
}
#[test]
fn test_p69_stream_flat_map() {
    assert_eq!(
        run_bootstrap_int("Phase69Test.class", "testStreamFlatMap", "()I"),
        15
    );
}
#[test]
fn test_p69_stream_peek() {
    assert_eq!(
        run_bootstrap_int("Phase69Test.class", "testStreamPeek", "()I"),
        9
    );
}
#[test]
fn test_p69_collectors_counting() {
    assert_eq!(
        run_bootstrap_int("Phase69Test.class", "testCollectorsCounting", "()I"),
        4
    );
}
#[test]
fn test_p69_optional_int_or_else() {
    assert_eq!(
        run_bootstrap_int("Phase69Test.class", "testOptionalInt", "()I"),
        10
    );
}
#[test]
fn test_p69_string_chars_count() {
    assert_eq!(
        run_bootstrap_int("Phase69Test.class", "testStringCharsCount", "()I"),
        3
    );
}

// Phase 70: String.join, Collections.unmodifiableList/singletonList, List.contains, Integer.compare
#[test]
fn test_p70_string_join() {
    assert_eq!(
        run_bootstrap_int("Phase70Test.class", "testStringJoin", "()I"),
        7
    );
}
#[test]
fn test_p70_string_join_list() {
    assert_eq!(
        run_bootstrap_int("Phase70Test.class", "testStringJoinList", "()I"),
        11
    );
}
#[test]
fn test_p70_unmodifiable_list() {
    assert_eq!(
        run_bootstrap_int("Phase70Test.class", "testUnmodifiableList", "()I"),
        3
    );
}
#[test]
fn test_p70_singleton_list() {
    assert_eq!(
        run_bootstrap_int("Phase70Test.class", "testSingletonList", "()I"),
        5
    );
}
#[test]
fn test_p70_list_contains() {
    assert_eq!(
        run_bootstrap_int("Phase70Test.class", "testListContains", "()I"),
        3
    );
}
#[test]
fn test_p70_string_format_mixed() {
    assert_eq!(
        run_bootstrap_int("Phase70Test.class", "testStringFormatMixed", "()I"),
        4
    );
}
#[test]
fn test_p70_math_max_chain() {
    assert_eq!(
        run_bootstrap_int("Phase70Test.class", "testMathMaxChain", "()I"),
        4
    );
}
#[test]
fn test_p70_integer_compare() {
    assert_eq!(
        run_bootstrap_int("Phase70Test.class", "testIntegerCompare", "()I"),
        7
    );
}

// Phase 71: Map.merge/compute/putIfAbsent, LinkedHashMap, List.indexOf/subList, Collections.reverse
#[test]
fn test_p71_map_get_or_default() {
    assert_eq!(
        run_bootstrap_int("Phase71Test.class", "testMapGetOrDefault", "()I"),
        100
    );
}
#[test]
fn test_p71_map_put_if_absent() {
    assert_eq!(
        run_bootstrap_int("Phase71Test.class", "testMapPutIfAbsent", "()I"),
        30
    );
}
#[test]
fn test_p71_map_merge() {
    assert_eq!(
        run_bootstrap_int("Phase71Test.class", "testMapMerge", "()I"),
        15
    );
}
#[test]
fn test_p71_map_compute() {
    assert_eq!(
        run_bootstrap_int("Phase71Test.class", "testMapCompute", "()I"),
        16
    );
}
#[test]
fn test_p71_linked_hash_map() {
    assert_eq!(
        run_bootstrap_int("Phase71Test.class", "testLinkedHashMap", "()I"),
        6
    );
}
#[test]
fn test_p71_list_index_of() {
    assert_eq!(
        run_bootstrap_int("Phase71Test.class", "testListIndexOf", "()I"),
        1
    );
}
#[test]
fn test_p71_list_sub_list() {
    assert_eq!(
        run_bootstrap_int("Phase71Test.class", "testListSubList", "()I"),
        12
    );
}
#[test]
fn test_p71_collections_reverse() {
    assert_eq!(
        run_bootstrap_int("Phase71Test.class", "testCollectionsReverse", "()I"),
        41
    );
}

// Phase 72: Stream.map+collect, filter+collect, IntStream.range/rangeClosed, count, anyMatch, allMatch/noneMatch
#[test]
fn test_p72_stream_to_list() {
    assert_eq!(
        run_bootstrap_int("Phase72Test.class", "testStreamToList", "()I"),
        15
    );
}
#[test]
fn test_p72_stream_filter_collect() {
    assert_eq!(
        run_bootstrap_int("Phase72Test.class", "testStreamFilterCollect", "()I"),
        5
    );
}
#[test]
fn test_p72_stream_map_collect() {
    assert_eq!(
        run_bootstrap_int("Phase72Test.class", "testStreamMapCollect", "()I"),
        13
    );
}
#[test]
fn test_p72_int_stream_range() {
    assert_eq!(
        run_bootstrap_int("Phase72Test.class", "testIntStreamRange", "()I"),
        15
    );
}
#[test]
fn test_p72_int_stream_range_closed() {
    assert_eq!(
        run_bootstrap_int("Phase72Test.class", "testIntStreamRangeClosed", "()I"),
        15
    );
}
#[test]
fn test_p72_stream_count() {
    assert_eq!(
        run_bootstrap_int("Phase72Test.class", "testStreamCount", "()I"),
        2
    );
}
#[test]
fn test_p72_stream_any_match() {
    assert_eq!(
        run_bootstrap_int("Phase72Test.class", "testStreamAnyMatch", "()I"),
        3
    );
}
#[test]
fn test_p72_stream_match_all() {
    assert_eq!(
        run_bootstrap_int("Phase72Test.class", "testStreamMatchAll", "()I"),
        3
    );
}

// Phase 73: Stream.reduce, min/max, Collectors.joining, findFirst, Optional, toSet
#[test]
fn test_p73_stream_reduce_identity() {
    assert_eq!(
        run_bootstrap_int("Phase73Test.class", "testStreamReduceIdentity", "()I"),
        10
    );
}
#[test]
fn test_p73_stream_reduce_optional() {
    assert_eq!(
        run_bootstrap_int("Phase73Test.class", "testStreamReduceOptional", "()I"),
        8
    );
}
#[test]
fn test_p73_stream_min_max() {
    assert_eq!(
        run_bootstrap_int("Phase73Test.class", "testStreamMinMax", "()I"),
        15
    );
}
#[test]
fn test_p73_collectors_joining() {
    assert_eq!(
        run_bootstrap_int("Phase73Test.class", "testCollectorsJoining", "()I"),
        12
    );
}
#[test]
fn test_p73_collectors_joining_prefix_suffix() {
    assert_eq!(
        run_bootstrap_int(
            "Phase73Test.class",
            "testCollectorsJoiningPrefixSuffix",
            "()I"
        ),
        9
    );
}
#[test]
fn test_p73_stream_find_first() {
    assert_eq!(
        run_bootstrap_int("Phase73Test.class", "testStreamFindFirst", "()I"),
        20
    );
}
#[test]
fn test_p73_optional_operations() {
    assert_eq!(
        run_bootstrap_int("Phase73Test.class", "testOptionalOperations", "()I"),
        8
    );
}
#[test]
fn test_p73_collectors_to_set() {
    assert_eq!(
        run_bootstrap_int("Phase73Test.class", "testCollectorsToSet", "()I"),
        3
    );
}

// Phase 74: TreeMap integer keys + headMap/tailMap, Queue/Stack, PriorityQueue, Map.entrySet, Collections.frequency/min/max
#[test]
fn test_p74_tree_map_ordered() {
    assert_eq!(
        run_bootstrap_int("Phase74Test.class", "testTreeMapOrdered", "()I"),
        13
    );
}
#[test]
fn test_p74_tree_map_head_tail() {
    assert_eq!(
        run_bootstrap_int("Phase74Test.class", "testTreeMapHeadTail", "()I"),
        23
    );
}
#[test]
fn test_p74_stack() {
    assert_eq!(
        run_bootstrap_int("Phase74Test.class", "testStack", "()I"),
        32
    );
}
#[test]
fn test_p74_queue() {
    assert_eq!(
        run_bootstrap_int("Phase74Test.class", "testQueue", "()I"),
        12
    );
}
#[test]
fn test_p74_priority_queue() {
    assert_eq!(
        run_bootstrap_int("Phase74Test.class", "testPriorityQueue", "()I"),
        13
    );
}
#[test]
fn test_p74_map_entry_set() {
    assert_eq!(
        run_bootstrap_int("Phase74Test.class", "testMapEntrySet", "()I"),
        6
    );
}
#[test]
fn test_p74_collections_frequency() {
    assert_eq!(
        run_bootstrap_int("Phase74Test.class", "testCollectionsFrequency", "()I"),
        3
    );
}
#[test]
fn test_p74_collections_min_max() {
    assert_eq!(
        run_bootstrap_int("Phase74Test.class", "testCollectionsMinMax", "()I"),
        18
    );
}

// Phase 75: String.chars() filter/sum/distinct, Arrays.stream(int[]), Arrays.asList, Collections.nCopies, forEach
#[test]
fn test_p75_string_chars_filter() {
    assert_eq!(
        run_bootstrap_int("Phase75Test.class", "testStringCharsFilter", "()I"),
        2
    );
}
#[test]
fn test_p75_string_chars_sum() {
    assert_eq!(
        run_bootstrap_int("Phase75Test.class", "testStringCharsSum", "()I"),
        4
    );
}
#[test]
fn test_p75_arrays_stream_int() {
    assert_eq!(
        run_bootstrap_int("Phase75Test.class", "testArraysStreamInt", "()I"),
        23
    );
}
#[test]
fn test_p75_arrays_stream_filter() {
    assert_eq!(
        run_bootstrap_int("Phase75Test.class", "testArraysStreamFilter", "()I"),
        3
    );
}
#[test]
fn test_p75_arrays_as_list() {
    assert_eq!(
        run_bootstrap_int("Phase75Test.class", "testArraysAsList", "()I"),
        4
    );
}
#[test]
fn test_p75_collections_n_copies() {
    assert_eq!(
        run_bootstrap_int("Phase75Test.class", "testCollectionsNCopies", "()I"),
        5
    );
}
#[test]
fn test_p75_iterable_for_each() {
    assert_eq!(
        run_bootstrap_int("Phase75Test.class", "testIterableForEach", "()I"),
        6
    );
}
#[test]
fn test_p75_string_chars_distinct() {
    assert_eq!(
        run_bootstrap_int("Phase75Test.class", "testStringCharsDistinct", "()I"),
        3
    );
}

// Phase 76: Comparator.comparing/reversed/naturalOrder/reverseOrder, Stream.sorted(Comparator), Map stream ops, Function.andThen
#[test]
fn test_p76_comparator_comparing() {
    assert_eq!(
        run_bootstrap_int("Phase76Test.class", "testComparatorComparing", "()I"),
        5
    );
}
#[test]
fn test_p76_comparator_reversed() {
    assert_eq!(
        run_bootstrap_int("Phase76Test.class", "testComparatorReversed", "()I"),
        6
    );
}
#[test]
fn test_p76_comparator_natural_order() {
    assert_eq!(
        run_bootstrap_int("Phase76Test.class", "testComparatorNaturalOrder", "()I"),
        15
    );
}
#[test]
fn test_p76_comparator_reverse_order() {
    assert_eq!(
        run_bootstrap_int("Phase76Test.class", "testComparatorReverseOrder", "()I"),
        51
    );
}
#[test]
fn test_p76_stream_sorted_comparator() {
    assert_eq!(
        run_bootstrap_int("Phase76Test.class", "testStreamSortedComparator", "()I"),
        13
    );
}
#[test]
fn test_p76_map_values_stream() {
    assert_eq!(
        run_bootstrap_int("Phase76Test.class", "testMapValuesStream", "()I"),
        60
    );
}
#[test]
fn test_p76_map_key_set_stream() {
    assert_eq!(
        run_bootstrap_int("Phase76Test.class", "testMapKeySetStream", "()I"),
        10
    );
}
#[test]
fn test_p76_function_compose() {
    assert_eq!(
        run_bootstrap_int("Phase76Test.class", "testFunctionCompose", "()I"),
        13
    );
}

// Phase 77: Pattern/Matcher (matches, find, group(n)), String.replaceAll/First, split regex, format padding
#[test]
fn test_p77_pattern_matches() {
    assert_eq!(
        run_bootstrap_int("Phase77Test.class", "testPatternMatches", "()I"),
        3
    );
}
#[test]
fn test_p77_string_matches() {
    assert_eq!(
        run_bootstrap_int("Phase77Test.class", "testStringMatches", "()I"),
        3
    );
}
#[test]
fn test_p77_pattern_matcher() {
    assert_eq!(
        run_bootstrap_int("Phase77Test.class", "testPatternMatcher", "()I"),
        3
    );
}
#[test]
fn test_p77_string_replace_all() {
    assert_eq!(
        run_bootstrap_int("Phase77Test.class", "testStringReplaceAll", "()I"),
        15
    );
}
#[test]
fn test_p77_string_replace_first() {
    assert_eq!(
        run_bootstrap_int("Phase77Test.class", "testStringReplaceFirst", "()I"),
        11
    );
}
#[test]
fn test_p77_string_split_regex() {
    assert_eq!(
        run_bootstrap_int("Phase77Test.class", "testStringSplitRegex", "()I"),
        3
    );
}
#[test]
fn test_p77_matcher_group_n() {
    assert_eq!(
        run_bootstrap_int("Phase77Test.class", "testMatcherGroup", "()I"),
        50
    );
}
#[test]
fn test_p77_string_format_padding() {
    assert_eq!(
        run_bootstrap_int("Phase77Test.class", "testStringFormatPadding", "()I"),
        5
    );
}

// Phase 78 probes
// ===========================================================================
// ---- Phase 78: Exception catching (NPE, AIOOB, CCE, StackOverflow) ----
// ===========================================================================

#[test]
fn test_p78_number_format_exception() {
    assert_eq!(
        run_bootstrap_int("Phase78Test.class", "testNumberFormatException", "()I"),
        42
    );
}
#[test]
fn test_p78_array_index_oob() {
    assert_eq!(
        run_bootstrap_int("Phase78Test.class", "testArrayIndexOutOfBounds", "()I"),
        99
    );
}
#[test]
fn test_p78_null_pointer() {
    assert_eq!(
        run_bootstrap_int("Phase78Test.class", "testNullPointerException", "()I"),
        7
    );
}
#[test]
fn test_p78_class_cast() {
    assert_eq!(
        run_bootstrap_int("Phase78Test.class", "testClassCastException", "()I"),
        55
    );
}
#[test]
fn test_p78_stack_overflow() {
    assert_eq!(
        run_bootstrap_int("Phase78Test.class", "testStackOverflow", "()I"),
        1
    );
}
#[test]
fn test_p78_finally_runs() {
    assert_eq!(
        run_bootstrap_int("Phase78Test.class", "testFinallyRuns", "()I"),
        111
    );
}
#[test]
fn test_p78_multi_catch() {
    assert_eq!(
        run_bootstrap_int("Phase78Test.class", "testMultiCatch", "()I"),
        3
    );
}
#[test]
fn test_p78_rethrow() {
    assert_eq!(
        run_bootstrap_int("Phase78Test.class", "testRethrow", "()I"),
        5
    );
}
// =========================================================================

// =========================================================================
// ---- Phase 79: List.of, Set.of, Map.of, java.util.Objects ----
// =========================================================================

#[test]
fn test_p79_list_of() {
    assert_eq!(
        run_bootstrap_int("Phase79Test.class", "testListOf", "()I"),
        5
    );
}
#[test]
fn test_p79_list_of_get() {
    assert_eq!(
        run_bootstrap_int("Phase79Test.class", "testListOfGet", "()I"),
        1
    );
}
#[test]
fn test_p79_list_of_empty() {
    assert_eq!(
        run_bootstrap_int("Phase79Test.class", "testListOfEmpty", "()I"),
        0
    );
}
#[test]
fn test_p79_list_of_contains() {
    assert_eq!(
        run_bootstrap_int("Phase79Test.class", "testListOfContains", "()I"),
        1
    );
}
#[test]
fn test_p79_set_of() {
    assert_eq!(
        run_bootstrap_int("Phase79Test.class", "testSetOf", "()I"),
        3
    );
}
#[test]
fn test_p79_set_of_contains() {
    assert_eq!(
        run_bootstrap_int("Phase79Test.class", "testSetOfContains", "()I"),
        1
    );
}
#[test]
fn test_p79_map_of() {
    assert_eq!(
        run_bootstrap_int("Phase79Test.class", "testMapOf", "()I"),
        3
    );
}
#[test]
fn test_p79_map_of_get() {
    assert_eq!(
        run_bootstrap_int("Phase79Test.class", "testMapOfGet", "()I"),
        42
    );
}
#[test]
fn test_p79_objects_require_non_null() {
    assert_eq!(
        run_bootstrap_int("Phase79Test.class", "testObjectsRequireNonNull", "()I"),
        1
    );
}
#[test]
fn test_p79_objects_require_non_null_pass() {
    assert_eq!(
        run_bootstrap_int("Phase79Test.class", "testObjectsRequireNonNullPass", "()I"),
        5
    );
}
#[test]
fn test_p79_objects_equals() {
    assert_eq!(
        run_bootstrap_int("Phase79Test.class", "testObjectsEquals", "()I"),
        15
    );
}
#[test]
fn test_p79_objects_is_null() {
    assert_eq!(
        run_bootstrap_int("Phase79Test.class", "testObjectsIsNull", "()I"),
        15
    );
}
#[test]
fn test_p79_objects_to_string() {
    assert_eq!(
        run_bootstrap_int("Phase79Test.class", "testObjectsToString", "()I"),
        7
    );
}
#[test]
fn test_p79_collections_empty() {
    assert_eq!(
        run_bootstrap_int("Phase79Test.class", "testCollectionsEmpty", "()I"),
        0
    );
}
#[test]
fn test_p79_singleton_list() {
    assert_eq!(
        run_bootstrap_int("Phase79Test.class", "testCollectionsSingletonList", "()I"),
        5
    );
}

// =========================================================================
// ---- Phase 80: Integer/Long bit ops, removeIf, forEach, Collections.swap/min/max ----
// =========================================================================

#[test]
fn test_p80_string_valueof_char() {
    assert_eq!(
        run_bootstrap_int("Phase80Test.class", "testStringValueOfChar", "()I"),
        1
    );
}
#[test]
fn test_p80_string_valueof_char_array() {
    assert_eq!(
        run_bootstrap_int("Phase80Test.class", "testStringValueOfCharArray", "()I"),
        2
    );
}
#[test]
fn test_p80_char_arithmetic() {
    assert_eq!(
        run_bootstrap_int("Phase80Test.class", "testCharArithmetic", "()I"),
        25
    );
}
#[test]
fn test_p80_char_boxing() {
    assert_eq!(
        run_bootstrap_int("Phase80Test.class", "testCharBoxing", "()I"),
        23
    );
}
#[test]
fn test_p80_integer_bitcount() {
    assert_eq!(
        run_bootstrap_int("Phase80Test.class", "testIntegerBitCount", "()I"),
        8
    );
}
#[test]
fn test_p80_integer_highest_one_bit() {
    assert_eq!(
        run_bootstrap_int("Phase80Test.class", "testIntegerHighestOneBit", "()I"),
        64
    );
}
#[test]
fn test_p80_integer_lowest_one_bit() {
    assert_eq!(
        run_bootstrap_int("Phase80Test.class", "testIntegerLowestOneBit", "()I"),
        4
    );
}
#[test]
fn test_p80_integer_leading_zeros() {
    assert_eq!(
        run_bootstrap_int(
            "Phase80Test.class",
            "testIntegerNumberOfLeadingZeros",
            "()I"
        ),
        31
    );
}
#[test]
fn test_p80_long_bitcount() {
    assert_eq!(
        run_bootstrap_int("Phase80Test.class", "testLongBitCount", "()I"),
        8
    );
}
#[test]
fn test_p80_math_long() {
    assert_eq!(
        run_bootstrap_int("Phase80Test.class", "testMathLong", "()I"),
        100
    );
}
#[test]
fn test_p80_foreach_lambda() {
    assert_eq!(
        run_bootstrap_int("Phase80Test.class", "testForEachLambda", "()I"),
        15
    );
}
#[test]
fn test_p80_remove_if() {
    assert_eq!(
        run_bootstrap_int("Phase80Test.class", "testRemoveIf", "()I"),
        3
    );
}
#[test]
fn test_p80_string_chars_count() {
    assert_eq!(
        run_bootstrap_int("Phase80Test.class", "testStringCharsCount", "()I"),
        3
    );
}
#[test]
fn test_p80_collections_swap() {
    assert_eq!(
        run_bootstrap_int("Phase80Test.class", "testCollectionsSwap", "()I"),
        30
    );
}
#[test]
fn test_p80_collections_min_max() {
    assert_eq!(
        run_bootstrap_int("Phase80Test.class", "testCollectionsMinMax", "()I"),
        8
    );
}

// =========================================================================
// ---- Phase 81: Custom exceptions, getCause, ArithmeticException, UnmodifiableList ----
// =========================================================================

#[test]
fn test_p81_custom_exception() {
    assert_eq!(
        run_bootstrap_int("Phase81Test.class", "testCustomException", "()I"),
        42
    );
}
#[test]
fn test_p81_exception_hierarchy() {
    assert_eq!(
        run_bootstrap_int("Phase81Test.class", "testExceptionHierarchy", "()I"),
        9
    );
}
#[test]
fn test_p81_exception_cause() {
    assert_eq!(
        run_bootstrap_int("Phase81Test.class", "testExceptionCause", "()I"),
        1
    );
}
#[test]
fn test_p81_exception_message() {
    assert_eq!(
        run_bootstrap_int("Phase81Test.class", "testExceptionMessage", "()I"),
        11
    );
}
#[test]
fn test_p81_division_by_zero() {
    assert_eq!(
        run_bootstrap_int("Phase81Test.class", "testDivisionByZero", "()I"),
        7
    );
}
#[test]
fn test_p81_unsupported_operation() {
    assert_eq!(
        run_bootstrap_int("Phase81Test.class", "testUnsupportedOperation", "()I"),
        1
    );
}
#[test]
fn test_p81_illegal_argument() {
    assert_eq!(
        run_bootstrap_int("Phase81Test.class", "testIllegalArgument", "()I"),
        7
    );
}
#[test]
fn test_p81_illegal_state() {
    assert_eq!(
        run_bootstrap_int("Phase81Test.class", "testIllegalState", "()I"),
        9
    );
}

// =========================================================================
// ---- Phase 82: 2D arrays, user Comparable, string switch, varargs ----
// =========================================================================

#[test]
fn test_p82_twodim_array() {
    assert_eq!(
        run_bootstrap_int("Phase82Test.class", "testTwoDimArray", "()I"),
        42
    );
}
#[test]
fn test_p82_twodim_array_sum() {
    assert_eq!(
        run_bootstrap_int("Phase82Test.class", "testTwoDimArraySum", "()I"),
        45
    );
}
#[test]
fn test_p82_twodim_array_length() {
    assert_eq!(
        run_bootstrap_int("Phase82Test.class", "testTwoDimArrayLength", "()I"),
        15
    );
}
#[test]
fn test_p82_user_comparable() {
    assert_eq!(
        run_bootstrap_int("Phase82Test.class", "testUserComparable", "()I"),
        13
    );
}
#[test]
fn test_p82_string_switch() {
    assert_eq!(
        run_bootstrap_int("Phase82Test.class", "testStringSwitch", "()I"),
        2
    );
}
#[test]
fn test_p82_string_switch_default() {
    assert_eq!(
        run_bootstrap_int("Phase82Test.class", "testStringSwitchDefault", "()I"),
        99
    );
}
#[test]
fn test_p82_varargs() {
    assert_eq!(
        run_bootstrap_int("Phase82Test.class", "testVarargs", "()I"),
        15
    );
}
#[test]
fn test_p82_varargs_empty() {
    assert_eq!(
        run_bootstrap_int("Phase82Test.class", "testVarargsEmpty", "()I"),
        0
    );
}
#[test]
fn test_p82_instanceof_chain() {
    assert_eq!(
        run_bootstrap_int("Phase82Test.class", "testInstanceofChain", "()I"),
        5
    );
}
#[test]
fn test_p82_ternary() {
    assert_eq!(
        run_bootstrap_int("Phase82Test.class", "testTernary", "()I"),
        10
    );
}

// =========================================================================
// ---- Phase 83: interface defaults, switch expr, records, pattern matching ----
// =========================================================================

#[test]
fn test_p83_interface_default() {
    assert_eq!(
        run_bootstrap_int("Phase83Test.class", "testInterfaceDefaultMethod", "()I"),
        11
    );
}
#[test]
fn test_p83_static_interface() {
    assert_eq!(
        run_bootstrap_int("Phase83Test.class", "testStaticInterfaceMethod", "()I"),
        49
    );
}
#[test]
fn test_p83_default_interface() {
    assert_eq!(
        run_bootstrap_int("Phase83Test.class", "testDefaultInterfaceMethod", "()I"),
        27
    );
}
#[test]
fn test_p83_switch_expression() {
    assert_eq!(
        run_bootstrap_int("Phase83Test.class", "testSwitchExpression", "()I"),
        1
    );
}
#[test]
fn test_p83_switch_yield() {
    assert_eq!(
        run_bootstrap_int("Phase83Test.class", "testSwitchExpressionYield", "()I"),
        25
    );
}
#[test]
fn test_p83_record() {
    assert_eq!(
        run_bootstrap_int("Phase83Test.class", "testRecord", "()I"),
        10
    );
}
#[test]
fn test_p83_record_method() {
    assert_eq!(
        run_bootstrap_int("Phase83Test.class", "testRecordMethod", "()I"),
        10
    );
}
#[test]
fn test_p83_pattern_instanceof() {
    assert_eq!(
        run_bootstrap_int("Phase83Test.class", "testPatternMatchingInstanceof", "()I"),
        42
    );
}
#[test]
fn test_p83_text_block() {
    assert_eq!(
        run_bootstrap_int("Phase83Test.class", "testTextBlock", "()I"),
        11
    );
}

// =========================================================================
// ---- Phase 84: Generics, BiFunction, Predicate, Consumer, Supplier ----
// =========================================================================

#[test]
fn test_p84_generic_class() {
    assert_eq!(
        run_bootstrap_int("Phase84Test.class", "testGenericClass", "()I"),
        47
    );
}
#[test]
fn test_p84_generic_method() {
    assert_eq!(
        run_bootstrap_int("Phase84Test.class", "testGenericMethod", "()I"),
        35
    );
}
#[test]
fn test_p84_functional_interface() {
    assert_eq!(
        run_bootstrap_int("Phase84Test.class", "testFunctionalInterface", "()I"),
        18
    );
}
#[test]
fn test_p84_bifunction() {
    assert_eq!(
        run_bootstrap_int("Phase84Test.class", "testBiFunction", "()I"),
        42
    );
}
#[test]
fn test_p84_function_compose() {
    assert_eq!(
        run_bootstrap_int("Phase84Test.class", "testFunctionCompose", "()I"),
        14
    );
}
#[test]
fn test_p84_predicate() {
    assert_eq!(
        run_bootstrap_int("Phase84Test.class", "testPredicate", "()I"),
        2
    );
}
#[test]
fn test_p84_consumer() {
    assert_eq!(
        run_bootstrap_int("Phase84Test.class", "testConsumer", "()I"),
        10
    );
}
#[test]
fn test_p84_supplier() {
    assert_eq!(
        run_bootstrap_int("Phase84Test.class", "testSupplier", "()I"),
        42
    );
}
#[test]
fn test_p84_unary_operator() {
    assert_eq!(
        run_bootstrap_int("Phase84Test.class", "testUnaryOperator", "()I"),
        6
    );
}

// =========================================================================
// ---- Phase 85: method refs, Stream.toList, abstract class, enum fields ----
// =========================================================================

#[test]
fn test_p85_bound_method_ref() {
    assert_eq!(
        run_bootstrap_int("Phase85Test.class", "testBoundMethodRef", "()I"),
        11
    );
}
#[test]
fn test_p85_bound_method_ref_on_arg() {
    assert_eq!(
        run_bootstrap_int("Phase85Test.class", "testBoundMethodRefOnArg", "()I"),
        1
    );
}
#[test]
fn test_p85_static_method_ref() {
    assert_eq!(
        run_bootstrap_int("Phase85Test.class", "testStaticMethodRef", "()I"),
        14
    );
}
#[test]
fn test_p85_unbound_method_ref() {
    assert_eq!(
        run_bootstrap_int("Phase85Test.class", "testUnboundMethodRef", "()I"),
        10
    );
}
#[test]
fn test_p85_stream_to_list() {
    assert_eq!(
        run_bootstrap_int("Phase85Test.class", "testStreamToList", "()I"),
        2
    );
}
#[test]
fn test_p85_nested_lambda() {
    assert_eq!(
        run_bootstrap_int("Phase85Test.class", "testNestedLambda", "()I"),
        42
    );
}
#[test]
fn test_p85_abstract_class() {
    assert_eq!(
        run_bootstrap_int("Phase85Test.class", "testAbstractClass", "()I"),
        74
    );
}
#[test]
fn test_p85_enum_with_fields() {
    assert_eq!(
        run_bootstrap_int("Phase85Test.class", "testEnumWithFields", "()I"),
        9
    );
}
#[test]
fn test_p85_static_initializer() {
    assert_eq!(
        run_bootstrap_int("Phase85Test.class", "testStaticInitializer", "()I"),
        100
    );
}
#[test]
fn test_p85_string_formatted() {
    assert_eq!(
        run_bootstrap_int("Phase85Test.class", "testStringFormatted", "()I"),
        9
    );
}

// ---- Phase 86 probes ----
#[test]
fn test_p86_varargs() {
    assert_eq!(
        run_bootstrap_int("Phase86Test.class", "testVarargs", "()I"),
        15
    );
}
#[test]
fn test_p86_comparable() {
    assert_eq!(
        run_bootstrap_int("Phase86Test.class", "testComparable", "()I"),
        1
    );
}
#[test]
fn test_p86_string_join() {
    assert_eq!(
        run_bootstrap_int("Phase86Test.class", "testStringJoin", "()I"),
        15
    );
}
#[test]
fn test_p86_optional() {
    assert_eq!(
        run_bootstrap_int("Phase86Test.class", "testOptional", "()I"),
        5
    );
}
#[test]
fn test_p86_optional_empty() {
    assert_eq!(
        run_bootstrap_int("Phase86Test.class", "testOptionalEmpty", "()I"),
        0
    );
}
#[test]
fn test_p86_custom_iterable() {
    assert_eq!(
        run_bootstrap_int("Phase86Test.class", "testCustomIterable", "()I"),
        15
    );
}
#[test]
fn test_p86_string_chars() {
    assert_eq!(
        run_bootstrap_int("Phase86Test.class", "testStringChars", "()I"),
        2
    );
}
#[test]
fn test_p86_collections_frequency() {
    assert_eq!(
        run_bootstrap_int("Phase86Test.class", "testCollectionsFrequency", "()I"),
        3
    );
}
#[test]
fn test_p86_treemap() {
    assert_eq!(
        run_bootstrap_int("Phase86Test.class", "testTreeMap", "()I"),
        1
    );
}
#[test]
fn test_p86_linked_list_deque() {
    assert_eq!(
        run_bootstrap_int("Phase86Test.class", "testLinkedListDeque", "()I"),
        4
    );
}

// ---- Phase 87 probes ----
#[test]
fn test_p87_stream_joining() {
    assert_eq!(
        run_bootstrap_int("Phase87Test.class", "testStreamJoining", "()I"),
        7
    );
}
#[test]
fn test_p87_stream_flatmap() {
    assert_eq!(
        run_bootstrap_int("Phase87Test.class", "testStreamFlatMap", "()I"),
        15
    );
}
#[test]
fn test_p87_stream_reduce() {
    assert_eq!(
        run_bootstrap_int("Phase87Test.class", "testStreamReduce", "()I"),
        15
    );
}
#[test]
fn test_p87_stream_sorted_comparator() {
    assert_eq!(
        run_bootstrap_int("Phase87Test.class", "testStreamSortedWithComparator", "()I"),
        5
    );
}
#[test]
fn test_p87_map_entry_sum() {
    assert_eq!(
        run_bootstrap_int("Phase87Test.class", "testMapEntrySum", "()I"),
        60
    );
}
#[test]
fn test_p87_instanceof_pattern() {
    assert_eq!(
        run_bootstrap_int("Phase87Test.class", "testInstanceof", "()I"),
        5
    );
}
#[test]
fn test_p87_switch_expression() {
    assert_eq!(
        run_bootstrap_int("Phase87Test.class", "testSwitchExpression", "()I"),
        9
    );
}
#[test]
fn test_p87_text_block() {
    assert_eq!(
        run_bootstrap_int("Phase87Test.class", "testTextBlock", "()I"),
        16
    );
}
#[test]
fn test_p87_multi_dim_array() {
    assert_eq!(
        run_bootstrap_int("Phase87Test.class", "testMultiDimArray", "()I"),
        45
    );
}
#[test]
fn test_p87_string_format_multi() {
    assert_eq!(
        run_bootstrap_int("Phase87Test.class", "testStringFormatMulti", "()I"),
        21
    );
}

// ---- Phase 88 probes ----
#[test]
fn test_p88_generic_pair() {
    assert_eq!(
        run_bootstrap_int("Phase88Test.class", "testGenericPair", "()I"),
        47
    );
}
#[test]
fn test_p88_stack_deque() {
    assert_eq!(
        run_bootstrap_int("Phase88Test.class", "testStackDeque", "()I"),
        5
    );
}
#[test]
fn test_p88_bitset() {
    assert_eq!(
        run_bootstrap_int("Phase88Test.class", "testBitSet", "()I"),
        3
    );
}
#[test]
fn test_p88_math_functions() {
    assert_eq!(
        run_bootstrap_int("Phase88Test.class", "testMathFunctions", "()I"),
        12
    );
}
#[test]
fn test_p88_character_methods() {
    assert_eq!(
        run_bootstrap_int("Phase88Test.class", "testCharacterMethods", "()I"),
        3
    );
}
#[test]
fn test_p88_string_split_join() {
    assert_eq!(
        run_bootstrap_int("Phase88Test.class", "testStringSplitJoin", "()I"),
        18
    );
}
#[test]
fn test_p88_interface_default() {
    assert_eq!(
        run_bootstrap_int("Phase88Test.class", "testInterfaceDefault", "()I"),
        13
    );
}
#[test]
fn test_p88_static_interface_method() {
    assert_eq!(
        run_bootstrap_int("Phase88Test.class", "testStaticInterfaceMethod", "()I"),
        1
    );
}
#[test]
fn test_p88_stream_collect_to_map() {
    assert_eq!(
        run_bootstrap_int("Phase88Test.class", "testStreamCollectToMap", "()I"),
        3
    );
}
#[test]
fn test_p88_exception_message() {
    assert_eq!(
        run_bootstrap_int("Phase88Test.class", "testExceptionMessage", "()I"),
        9
    );
}
#[test]
fn test_p89_map_put_if_absent() {
    assert_eq!(
        run_bootstrap_int("Phase89Test.class", "testMapPutIfAbsent", "()I"),
        3
    );
}
#[test]
fn test_p89_map_merge() {
    assert_eq!(
        run_bootstrap_int("Phase89Test.class", "testMapMerge", "()I"),
        22
    );
}
#[test]
fn test_p89_comparator_comparing() {
    assert_eq!(
        run_bootstrap_int("Phase89Test.class", "testComparatorComparing", "()I"),
        9
    );
}
#[test]
fn test_p89_collections_frequency() {
    assert_eq!(
        run_bootstrap_int("Phase89Test.class", "testCollectionsFrequency", "()I"),
        3
    );
}
#[test]
fn test_p89_string_chars_stream() {
    assert_eq!(
        run_bootstrap_int("Phase89Test.class", "testStringCharsStream", "()I"),
        3
    );
}
#[test]
fn test_p89_list_contains() {
    assert_eq!(
        run_bootstrap_int("Phase89Test.class", "testListContains", "()I"),
        15
    );
}
#[test]
fn test_p89_optional_map() {
    assert_eq!(
        run_bootstrap_int("Phase89Test.class", "testOptionalMap", "()I"),
        5
    );
}
#[test]
fn test_p89_string_value_of() {
    assert_eq!(
        run_bootstrap_int("Phase89Test.class", "testStringValueOf", "()I"),
        10
    );
}
#[test]
fn test_p89_int_stream_range() {
    assert_eq!(
        run_bootstrap_int("Phase89Test.class", "testIntStreamRange", "()I"),
        15
    );
}
#[test]
fn test_p89_map_for_each() {
    assert_eq!(
        run_bootstrap_int("Phase89Test.class", "testMapForEach", "()I"),
        6
    );
}
#[test]
fn test_p90_char_array_sum() {
    assert_eq!(
        run_bootstrap_int("Phase90Test.class", "testCharArraySum", "()I"),
        3
    );
}
#[test]
fn test_p90_arrays_as_list_stream() {
    assert_eq!(
        run_bootstrap_int("Phase90Test.class", "testArraysAsListStream", "()I"),
        15
    );
}
#[test]
fn test_p90_collections_reverse() {
    assert_eq!(
        run_bootstrap_int("Phase90Test.class", "testCollectionsReverse", "()I"),
        6
    );
}
#[test]
fn test_p90_collections_min_max() {
    assert_eq!(
        run_bootstrap_int("Phase90Test.class", "testCollectionsMinMax", "()I"),
        10
    );
}
#[test]
fn test_p90_substring_chain() {
    assert_eq!(
        run_bootstrap_int("Phase90Test.class", "testSubstringChain", "()I"),
        10
    );
}
#[test]
fn test_p90_static_fields() {
    assert_eq!(
        run_bootstrap_int("Phase90Test.class", "testStaticFields", "()I"),
        101
    );
}
#[test]
fn test_p90_list_set() {
    assert_eq!(
        run_bootstrap_int("Phase90Test.class", "testListSet", "()I"),
        139
    );
}
#[test]
fn test_p90_iterator_sum() {
    assert_eq!(
        run_bootstrap_int("Phase90Test.class", "testIteratorSum", "()I"),
        50
    );
}
#[test]
fn test_p90_ternary_in_stream() {
    assert_eq!(
        run_bootstrap_int("Phase90Test.class", "testTernaryInStream", "()I"),
        33
    );
}
#[test]
fn test_p90_string_equality() {
    assert_eq!(
        run_bootstrap_int("Phase90Test.class", "testStringEquality", "()I"),
        111
    );
}
#[test]
fn test_p91_multi_level_inheritance() {
    assert_eq!(
        run_bootstrap_int("Phase91Test.class", "testMultiLevelInheritance", "()I"),
        15
    );
}
#[test]
fn test_p91_interface_default_override() {
    assert_eq!(
        run_bootstrap_int("Phase91Test.class", "testInterfaceDefaultOverride", "()I"),
        21
    );
}
#[test]
fn test_p91_integer_overflow() {
    assert_eq!(
        run_bootstrap_int("Phase91Test.class", "testIntegerOverflow", "()I"),
        1
    );
}
#[test]
fn test_p91_long_arithmetic() {
    assert_eq!(
        run_bootstrap_int("Phase91Test.class", "testLongArithmetic", "()I"),
        3
    );
}
#[test]
fn test_p91_nested_class_outer() {
    assert_eq!(
        run_bootstrap_int("Phase91Test.class", "testNestedClassAccessesOuter", "()I"),
        42
    );
}
#[test]
fn test_p91_stream_distinct() {
    assert_eq!(
        run_bootstrap_int("Phase91Test.class", "testStreamDistinct", "()I"),
        4
    );
}
#[test]
fn test_p91_stream_limit() {
    assert_eq!(
        run_bootstrap_int("Phase91Test.class", "testStreamLimit", "()I"),
        45
    );
}
#[test]
fn test_p91_map_entry_set() {
    assert_eq!(
        run_bootstrap_int("Phase91Test.class", "testMapEntrySet", "()I"),
        6
    );
}
#[test]
fn test_p91_fibonacci() {
    assert_eq!(
        run_bootstrap_int("Phase91Test.class", "testFibonacci", "()I"),
        55
    );
}
#[test]
fn test_p91_list_sub_list() {
    assert_eq!(
        run_bootstrap_int("Phase91Test.class", "testListSubList", "()I"),
        90
    );
}
#[test]
fn test_p92_varargs() {
    assert_eq!(
        run_bootstrap_int("Phase92Test.class", "testVarargs", "()I"),
        15
    );
}
#[test]
fn test_p92_string_format_args() {
    assert_eq!(
        run_bootstrap_int("Phase92Test.class", "testStringFormatArgs", "()I"),
        28
    );
}
#[test]
fn test_p92_chained_stream() {
    assert_eq!(
        run_bootstrap_int("Phase92Test.class", "testChainedStream", "()I"),
        220
    );
}
#[test]
fn test_p92_hashmap_values() {
    assert_eq!(
        run_bootstrap_int("Phase92Test.class", "testHashMapValues", "()I"),
        267
    );
}
#[test]
fn test_p92_string_split_process() {
    assert_eq!(
        run_bootstrap_int("Phase92Test.class", "testStringSplitProcess", "()I"),
        150
    );
}
#[test]
fn test_p92_do_while() {
    assert_eq!(
        run_bootstrap_int("Phase92Test.class", "testDoWhile", "()I"),
        120
    );
}
#[test]
fn test_p92_labeled_break() {
    assert_eq!(
        run_bootstrap_int("Phase92Test.class", "testLabeledBreak", "()I"),
        9
    );
}
#[test]
fn test_p92_array_of_objects() {
    assert_eq!(
        run_bootstrap_int("Phase92Test.class", "testArrayOfObjects", "()I"),
        15
    );
}
#[test]
fn test_p92_grouping_by() {
    assert_eq!(
        run_bootstrap_int("Phase92Test.class", "testGroupingBy", "()I"),
        3
    );
}
#[test]
fn test_p92_string_format_char() {
    assert_eq!(
        run_bootstrap_int("Phase92Test.class", "testStringFormatChar", "()I"),
        16
    );
}
#[test]
fn test_p93_deque_as_stack() {
    assert_eq!(
        run_bootstrap_int("Phase93Test.class", "testDequeAsStack", "()I"),
        6
    );
}
#[test]
fn test_p93_deque_as_queue() {
    assert_eq!(
        run_bootstrap_int("Phase93Test.class", "testDequeAsQueue", "()I"),
        12
    );
}
#[test]
fn test_p93_stream_peek() {
    assert_eq!(
        run_bootstrap_int("Phase93Test.class", "testStreamPeek", "()I"),
        20
    );
}
#[test]
fn test_p93_collectors_joining() {
    assert_eq!(
        run_bootstrap_int("Phase93Test.class", "testCollectorsJoining", "()I"),
        12
    );
}
#[test]
fn test_p93_compute_if_absent() {
    assert_eq!(
        run_bootstrap_int("Phase93Test.class", "testComputeIfAbsent", "()I"),
        3
    );
}
#[test]
fn test_p93_instanceof() {
    assert_eq!(
        run_bootstrap_int("Phase93Test.class", "testInstanceOf", "()I"),
        111
    );
}
#[test]
fn test_p93_conditional_chain() {
    assert_eq!(
        run_bootstrap_int("Phase93Test.class", "testConditionalChain", "()I"),
        6
    );
}
#[test]
fn test_p93_array_sort_search() {
    assert_eq!(
        run_bootstrap_int("Phase93Test.class", "testArraySortSearch", "()I"),
        9
    );
}
#[test]
fn test_p93_string_join_list() {
    assert_eq!(
        run_bootstrap_int("Phase93Test.class", "testStringJoinList", "()I"),
        16
    );
}
#[test]
fn test_p93_multiple_returns() {
    assert_eq!(
        run_bootstrap_int("Phase93Test.class", "testMultipleReturns", "()I"),
        5
    );
}
#[test]
fn test_p94_collectors_counting() {
    assert_eq!(
        run_bootstrap_int("Phase94Test.class", "testCollectorsCounting", "()I"),
        3
    );
}
#[test]
fn test_p94_intstream_map_to_obj() {
    assert_eq!(
        run_bootstrap_int("Phase94Test.class", "testIntStreamMapToObj", "()I"),
        7
    );
}
#[test]
fn test_p94_comparable_impl() {
    assert_eq!(
        run_bootstrap_int("Phase94Test.class", "testComparableImpl", "()I"),
        15
    );
}
#[test]
fn test_p94_map_get_or_default() {
    assert_eq!(
        run_bootstrap_int("Phase94Test.class", "testMapGetOrDefault", "()I"),
        15
    );
}
#[test]
fn test_p94_string_contains() {
    assert_eq!(
        run_bootstrap_int("Phase94Test.class", "testStringContains", "()I"),
        101
    );
}
#[test]
fn test_p94_2d_array() {
    assert_eq!(
        run_bootstrap_int("Phase94Test.class", "test2DArray", "()I"),
        45
    );
}
#[test]
fn test_p94_enum_ordinal() {
    assert_eq!(
        run_bootstrap_int("Phase94Test.class", "testEnumOrdinal", "()I"),
        6
    );
}
#[test]
fn test_p94_while_complex() {
    assert_eq!(
        run_bootstrap_int("Phase94Test.class", "testWhileComplex", "()I"),
        25
    );
}
#[test]
fn test_p94_map_remove() {
    assert_eq!(
        run_bootstrap_int("Phase94Test.class", "testMapRemove", "()I"),
        2
    );
}
#[test]
fn test_p94_stream_boxed_sum() {
    assert_eq!(
        run_bootstrap_int("Phase94Test.class", "testStreamBoxedSum", "()I"),
        24
    );
}
#[test]
fn test_p95_stream_generate() {
    assert_eq!(
        run_bootstrap_int("Phase95Test.class", "testStreamGenerate", "()I"),
        10
    );
}
#[test]
fn test_p95_optional_is_present() {
    assert_eq!(
        run_bootstrap_int("Phase95Test.class", "testOptionalIsPresent", "()I"),
        11
    );
}
#[test]
fn test_p95_map_keyset() {
    assert_eq!(
        run_bootstrap_int("Phase95Test.class", "testMapKeySet", "()I"),
        6
    );
}
#[test]
fn test_p95_arrays_stream_obj() {
    assert_eq!(
        run_bootstrap_int("Phase95Test.class", "testArraysStreamObj", "()I"),
        2
    );
}
#[test]
fn test_p95_math_ops() {
    assert_eq!(
        run_bootstrap_int("Phase95Test.class", "testMathOps", "()I"),
        45
    );
}
#[test]
fn test_p95_string_builder_chain() {
    assert_eq!(
        run_bootstrap_int("Phase95Test.class", "testStringBuilderChain", "()I"),
        13
    );
}
#[test]
fn test_p95_try_finally_return() {
    assert_eq!(
        run_bootstrap_int("Phase95Test.class", "testTryFinallyReturn", "()I"),
        42
    );
}
#[test]
fn test_p95_nested_map() {
    assert_eq!(
        run_bootstrap_int("Phase95Test.class", "testNestedMap", "()I"),
        30
    );
}
#[test]
fn test_p95_stream_matching() {
    assert_eq!(
        run_bootstrap_int("Phase95Test.class", "testStreamMatching", "()I"),
        111
    );
}
#[test]
fn test_p95_string_replace_all() {
    assert_eq!(
        run_bootstrap_int("Phase95Test.class", "testStringReplaceAll", "()I"),
        16
    );
}
#[test]
fn test_p96_linked_list_recursion() {
    assert_eq!(
        run_bootstrap_int("Phase96Test.class", "testLinkedListRecursion", "()I"),
        15
    );
}
#[test]
fn test_p96_generic_method() {
    assert_eq!(
        run_bootstrap_int("Phase96Test.class", "testGenericMethod", "()I"),
        26
    );
}
#[test]
fn test_p96_exception_cause() {
    assert_eq!(
        run_bootstrap_int("Phase96Test.class", "testExceptionCause", "()I"),
        12
    );
}
#[test]
fn test_p96_bitwise_ops() {
    assert_eq!(
        run_bootstrap_int("Phase96Test.class", "testBitwiseOps", "()I"),
        76
    );
}
#[test]
fn test_p96_shift_ops() {
    assert_eq!(
        run_bootstrap_int("Phase96Test.class", "testShiftOps", "()I"),
        36
    );
}
#[test]
fn test_p96_collect_then_sort() {
    assert_eq!(
        run_bootstrap_int("Phase96Test.class", "testCollectThenSort", "()I"),
        9
    );
}
#[test]
fn test_p96_string_matches() {
    assert_eq!(
        run_bootstrap_int("Phase96Test.class", "testStringMatches", "()I"),
        2
    );
}
#[test]
fn test_p96_multi_dim_sum() {
    assert_eq!(
        run_bootstrap_int("Phase96Test.class", "testMultiDimSum", "()I"),
        45
    );
}
#[test]
fn test_p96_instanceof_cast() {
    assert_eq!(
        run_bootstrap_int("Phase96Test.class", "testInstanceOfCast", "()I"),
        50
    );
}
#[test]
fn test_p96_map_accumulation() {
    assert_eq!(
        run_bootstrap_int("Phase96Test.class", "testMapAccumulation", "()I"),
        5
    );
}
#[test]
fn test_p97_record_shapes() {
    assert_eq!(
        run_bootstrap_int("Phase97Test.class", "testRecordShapes", "()I"),
        33
    );
}
#[test]
fn test_p97_map_compute() {
    assert_eq!(
        run_bootstrap_int("Phase97Test.class", "testMapCompute", "()I"),
        5
    );
}
#[test]
fn test_p97_stream_take_while() {
    assert_eq!(
        run_bootstrap_int("Phase97Test.class", "testStreamTakeWhile", "()I"),
        3
    );
}
#[test]
fn test_p97_stream_drop_while() {
    assert_eq!(
        run_bootstrap_int("Phase97Test.class", "testStreamDropWhile", "()I"),
        12
    );
}
#[test]
fn test_p97_n_copies() {
    assert_eq!(
        run_bootstrap_int("Phase97Test.class", "testNCopies", "()I"),
        5
    );
}
#[test]
fn test_p97_string_case() {
    assert_eq!(
        run_bootstrap_int("Phase97Test.class", "testStringCase", "()I"),
        1
    );
}
#[test]
fn test_p97_intstream_average() {
    assert_eq!(
        run_bootstrap_int("Phase97Test.class", "testIntStreamAverage", "()I"),
        6
    );
}
#[test]
fn test_p97_treemap_ordering() {
    assert_eq!(
        run_bootstrap_int("Phase97Test.class", "testTreeMapOrdering", "()I"),
        6
    );
}
#[test]
fn test_p97_long_stream_ops() {
    assert_eq!(
        run_bootstrap_int("Phase97Test.class", "testLongStreamOps", "()I"),
        15
    );
}
#[test]
fn test_p97_string_repeat() {
    assert_eq!(
        run_bootstrap_int("Phase97Test.class", "testStringRepeat", "()I"),
        8
    );
}
#[test]
fn test_p98_double_stream() {
    assert_eq!(
        run_bootstrap_int("Phase98Test.class", "testDoubleStream", "()I"),
        9
    );
}
#[test]
fn test_p98_map_replace_all() {
    assert_eq!(
        run_bootstrap_int("Phase98Test.class", "testMapReplaceAll", "()I"),
        60
    );
}
#[test]
fn test_p98_grouping_by_count() {
    assert_eq!(
        run_bootstrap_int("Phase98Test.class", "testGroupingByCount", "()I"),
        6
    );
}
#[test]
fn test_p98_stream_flat_map_int() {
    assert_eq!(
        run_bootstrap_int("Phase98Test.class", "testStreamFlatMapInt", "()I"),
        324
    );
}
#[test]
fn test_p98_comparator_reversed() {
    assert_eq!(
        run_bootstrap_int("Phase98Test.class", "testComparatorReversed", "()I"),
        10
    );
}
#[test]
fn test_p98_multiple_interfaces() {
    assert_eq!(
        run_bootstrap_int("Phase98Test.class", "testMultipleInterfaces", "()I"),
        21
    );
}
#[test]
fn test_p98_string_index_of() {
    assert_eq!(
        run_bootstrap_int("Phase98Test.class", "testStringIndexOf", "()I"),
        24
    );
}
#[test]
fn test_p98_exception_hierarchy() {
    assert_eq!(
        run_bootstrap_int("Phase98Test.class", "testExceptionHierarchy", "()I"),
        21
    );
}
#[test]
fn test_p98_nested_lambda_capture() {
    assert_eq!(
        run_bootstrap_int("Phase98Test.class", "testNestedLambdaCapture", "()I"),
        108
    );
}
#[test]
fn test_p98_map_values_stream() {
    assert_eq!(
        run_bootstrap_int("Phase98Test.class", "testMapValuesStream", "()I"),
        2
    );
}
#[test]
fn test_p99_priority_queue() {
    assert_eq!(
        run_bootstrap_int("Phase99Test.class", "testPriorityQueue", "()I"),
        35
    );
}
#[test]
fn test_p99_string_format_padding() {
    assert_eq!(
        run_bootstrap_int("Phase99Test.class", "testStringFormatPadding", "()I"),
        5
    );
}
#[test]
fn test_p99_method_chaining() {
    assert_eq!(
        run_bootstrap_int("Phase99Test.class", "testMethodChaining", "()I"),
        10
    );
}
#[test]
fn test_p99_iterable_for_each() {
    assert_eq!(
        run_bootstrap_int("Phase99Test.class", "testIterableForEach", "()I"),
        15
    );
}
#[test]
fn test_p99_map_entryset_stream() {
    assert_eq!(
        run_bootstrap_int("Phase99Test.class", "testMapEntrySetStream", "()I"),
        6
    );
}
#[test]
fn test_p99_arrays_copy_of_range() {
    assert_eq!(
        run_bootstrap_int("Phase99Test.class", "testArraysCopyOfRange", "()I"),
        90
    );
}
#[test]
fn test_p99_partitioning_by() {
    assert_eq!(
        run_bootstrap_int("Phase99Test.class", "testPartitioningBy", "()I"),
        10
    );
}
#[test]
fn test_p99_char_array_conversion() {
    assert_eq!(
        run_bootstrap_int("Phase99Test.class", "testCharArrayConversion", "()I"),
        5
    );
}
#[test]
fn test_p99_stream_min_max() {
    assert_eq!(
        run_bootstrap_int("Phase99Test.class", "testStreamMinMax", "()I"),
        10
    );
}
#[test]
fn test_p99_integer_radix_strings() {
    assert_eq!(
        run_bootstrap_int("Phase99Test.class", "testIntegerRadixStrings", "()I"),
        6
    );
}

#[test]
fn test_p100_function_compose() {
    assert_eq!(
        run_bootstrap_int("Phase100Test.class", "testFunctionCompose", "()I"),
        29
    );
}

#[test]
fn test_p100_predicate_compose() {
    assert_eq!(
        run_bootstrap_int("Phase100Test.class", "testPredicateCompose", "()I"),
        9
    );
}

#[test]
fn test_p100_consumer_and_then() {
    assert_eq!(
        run_bootstrap_int("Phase100Test.class", "testConsumerAndThen", "()I"),
        9
    );
}

#[test]
fn test_p100_supplier_get() {
    assert_eq!(
        run_bootstrap_int("Phase100Test.class", "testSupplierGet", "()I"),
        13
    );
}

#[test]
fn test_p100_bifunction() {
    assert_eq!(
        run_bootstrap_int("Phase100Test.class", "testBiFunction", "()I"),
        6
    );
}

#[test]
fn test_p100_map_foreach_biconsumer() {
    assert_eq!(
        run_bootstrap_int("Phase100Test.class", "testMapForEachBiConsumer", "()I"),
        30
    );
}

#[test]
fn test_p100_collectors_summarizing_int() {
    assert_eq!(
        run_bootstrap_int("Phase100Test.class", "testCollectorsSummarizingInt", "()I"),
        21
    );
}

#[test]
fn test_p100_stream_map_to_long() {
    assert_eq!(
        run_bootstrap_int("Phase100Test.class", "testStreamMapToLong", "()I"),
        10
    );
}

#[test]
fn test_p100_unmodifiable_list() {
    assert_eq!(
        run_bootstrap_int("Phase100Test.class", "testUnmodifiableList", "()I"),
        13
    );
}

#[test]
fn test_p100_intstream_method_ref() {
    assert_eq!(
        run_bootstrap_int("Phase100Test.class", "testIntStreamMethodRef", "()I"),
        21
    );
}

#[test]
fn test_p101_comparator_then_comparing() {
    assert_eq!(
        run_bootstrap_int("Phase101Test.class", "testComparatorThenComparing", "()I"),
        11
    );
}

#[test]
fn test_p101_list_sublist() {
    assert_eq!(
        run_bootstrap_int("Phase101Test.class", "testListSubList", "()I"),
        90
    );
}

#[test]
fn test_p101_collections_reverse() {
    assert_eq!(
        run_bootstrap_int("Phase101Test.class", "testCollectionsReverse", "()I"),
        6
    );
}

#[test]
fn test_p101_collections_frequency() {
    assert_eq!(
        run_bootstrap_int("Phase101Test.class", "testCollectionsFrequency", "()I"),
        3
    );
}

#[test]
fn test_p101_map_merge() {
    assert_eq!(
        run_bootstrap_int("Phase101Test.class", "testMapMerge", "()I"),
        16
    );
}

#[test]
fn test_p101_stream_distinct() {
    assert_eq!(
        run_bootstrap_int("Phase101Test.class", "testStreamDistinct", "()I"),
        4
    );
}

#[test]
fn test_p101_optional_filter() {
    assert_eq!(
        run_bootstrap_int("Phase101Test.class", "testOptionalFilter", "()I"),
        9
    );
}

#[test]
fn test_p101_string_chars_stream() {
    assert_eq!(
        run_bootstrap_int("Phase101Test.class", "testStringCharsStream", "()I"),
        3
    );
}

#[test]
fn test_p101_arrays_stream_int() {
    assert_eq!(
        run_bootstrap_int("Phase101Test.class", "testArraysStreamInt", "()I"),
        6
    );
}

#[test]
fn test_p101_linked_list_deque() {
    assert_eq!(
        run_bootstrap_int("Phase101Test.class", "testLinkedListDeque", "()I"),
        5
    );
}

#[test]
fn test_p102_map_put_if_absent() {
    assert_eq!(
        run_bootstrap_int("Phase102Test.class", "testMapPutIfAbsent", "()I"),
        3
    );
}

#[test]
fn test_p102_stream_sorted_comparator() {
    assert_eq!(
        run_bootstrap_int("Phase102Test.class", "testStreamSortedComparator", "()I"),
        4
    );
}

#[test]
fn test_p102_intstream_range_sum() {
    assert_eq!(
        run_bootstrap_int("Phase102Test.class", "testIntStreamRangeSum", "()I"),
        55
    );
}

#[test]
fn test_p102_collections_min_max() {
    assert_eq!(
        run_bootstrap_int("Phase102Test.class", "testCollectionsMinMax", "()I"),
        10
    );
}

#[test]
fn test_p102_list_index_of() {
    assert_eq!(
        run_bootstrap_int("Phase102Test.class", "testListIndexOf", "()I"),
        4
    );
}

#[test]
fn test_p102_stringbuilder_insert() {
    assert_eq!(
        run_bootstrap_int("Phase102Test.class", "testStringBuilderInsert", "()I"),
        12
    );
}

#[test]
fn test_p102_string_formatted() {
    assert_eq!(
        run_bootstrap_int("Phase102Test.class", "testStringFormatted", "()I"),
        9
    );
}

#[test]
fn test_p102_map_contains_value() {
    assert_eq!(
        run_bootstrap_int("Phase102Test.class", "testMapContainsValue", "()I"),
        2
    );
}

#[test]
fn test_p102_stream_none_match() {
    assert_eq!(
        run_bootstrap_int("Phase102Test.class", "testStreamNoneMatch", "()I"),
        11
    );
}

#[test]
fn test_p102_arrays_sort_objects() {
    assert_eq!(
        run_bootstrap_int("Phase102Test.class", "testArraysSortObjects", "()I"),
        9
    );
}

#[test]
fn test_p103_stack() {
    assert_eq!(
        run_bootstrap_int("Phase103Test.class", "testStack", "()I"),
        8
    );
}

#[test]
fn test_p103_collections_shuffle() {
    assert_eq!(
        run_bootstrap_int("Phase103Test.class", "testCollectionsShuffle", "()I"),
        5
    );
}

#[test]
fn test_p103_stream_generate() {
    assert_eq!(
        run_bootstrap_int("Phase103Test.class", "testStreamGenerate", "()I"),
        15
    );
}

#[test]
fn test_p103_string_join_list() {
    assert_eq!(
        run_bootstrap_int("Phase103Test.class", "testStringJoinList", "()I"),
        7
    );
}

#[test]
fn test_p103_map_get_or_default() {
    assert_eq!(
        run_bootstrap_int("Phase103Test.class", "testMapGetOrDefault", "()I"),
        109
    );
}

#[test]
fn test_p103_tree_set() {
    assert_eq!(
        run_bootstrap_int("Phase103Test.class", "testTreeSet", "()I"),
        11
    );
}

#[test]
fn test_p103_intstream_range_closed() {
    assert_eq!(
        run_bootstrap_int("Phase103Test.class", "testIntStreamRangeClosed", "()I"),
        55
    );
}

#[test]
fn test_p103_comparator_reversed_method_ref() {
    assert_eq!(
        run_bootstrap_int(
            "Phase103Test.class",
            "testComparatorReversedOnMethodRef",
            "()I"
        ),
        6
    );
}

#[test]
fn test_p103_iterator_remove() {
    assert_eq!(
        run_bootstrap_int("Phase103Test.class", "testIteratorRemove", "()I"),
        3
    );
}

#[test]
fn test_p103_character_digit() {
    assert_eq!(
        run_bootstrap_int("Phase103Test.class", "testCharacterDigit", "()I"),
        13
    );
}

#[test]
fn test_p104_map_entryset_iteration() {
    assert_eq!(
        run_bootstrap_int("Phase104Test.class", "testMapEntrySetIteration", "()I"),
        6
    );
}

#[test]
fn test_p104_stream_sorted_natural() {
    assert_eq!(
        run_bootstrap_int("Phase104Test.class", "testStreamSortedNatural", "()I"),
        1
    );
}

#[test]
fn test_p104_optional_of_nullable() {
    assert_eq!(
        run_bootstrap_int("Phase104Test.class", "testOptionalOfNullable", "()I"),
        5
    );
}

#[test]
fn test_p104_list_of() {
    assert_eq!(
        run_bootstrap_int("Phase104Test.class", "testListOf", "()I"),
        15
    );
}

#[test]
fn test_p104_map_of() {
    assert_eq!(
        run_bootstrap_int("Phase104Test.class", "testMapOf", "()I"),
        60
    );
}

#[test]
fn test_p104_set_of() {
    assert_eq!(
        run_bootstrap_int("Phase104Test.class", "testSetOf", "()I"),
        5
    );
}

#[test]
fn test_p104_string_valueof_char() {
    assert_eq!(
        run_bootstrap_int("Phase104Test.class", "testStringValueOfChar", "()I"),
        11
    );
}

#[test]
fn test_p104_stream_collect_joining() {
    assert_eq!(
        run_bootstrap_int("Phase104Test.class", "testStreamCollectJoining", "()I"),
        12
    );
}

#[test]
fn test_p104_collections_swap() {
    assert_eq!(
        run_bootstrap_int("Phase104Test.class", "testCollectionsSwap", "()I"),
        6
    );
}

#[test]
fn test_p104_arrays_equals() {
    assert_eq!(
        run_bootstrap_int("Phase104Test.class", "testArraysEquals", "()I"),
        1
    );
}

#[test]
fn test_p105_varargs() {
    assert_eq!(
        run_bootstrap_int("Phase105Test.class", "testVarargs", "()I"),
        15
    );
}

#[test]
fn test_p105_string_chars_map_to_obj() {
    assert_eq!(
        run_bootstrap_int("Phase105Test.class", "testStringCharsMapToObj", "()I"),
        2
    );
}

#[test]
fn test_p105_collectors_to_unmodifiable_list() {
    assert_eq!(
        run_bootstrap_int(
            "Phase105Test.class",
            "testCollectorsToUnmodifiableList",
            "()I"
        ),
        25
    );
}

#[test]
fn test_p105_map_compute_new() {
    assert_eq!(
        run_bootstrap_int("Phase105Test.class", "testMapComputeNew", "()I"),
        21
    );
}

#[test]
fn test_p105_integer_compare() {
    assert_eq!(
        run_bootstrap_int("Phase105Test.class", "testIntegerCompare", "()I"),
        3
    );
}

#[test]
fn test_p105_string_format_multiple_args() {
    assert_eq!(
        run_bootstrap_int("Phase105Test.class", "testStringFormatMultipleArgs", "()I"),
        16
    );
}

#[test]
fn test_p105_collect_to_map() {
    assert_eq!(
        run_bootstrap_int("Phase105Test.class", "testCollectToMap", "()I"),
        6
    );
}

#[test]
fn test_p105_optional_int() {
    assert_eq!(
        run_bootstrap_int("Phase105Test.class", "testOptionalInt", "()I"),
        50
    );
}

#[test]
fn test_p105_longstream_range() {
    assert_eq!(
        run_bootstrap_int("Phase105Test.class", "testLongStreamRange", "()I"),
        15
    );
}

#[test]
fn test_p105_collections_fill() {
    assert_eq!(
        run_bootstrap_int("Phase105Test.class", "testCollectionsFill", "()I"),
        1
    );
}

#[test]
fn test_p106_nested_collections() {
    assert_eq!(
        run_bootstrap_int("Phase106Test.class", "testNestedCollections", "()I"),
        21
    );
}

#[test]
fn test_p106_stream_flat_map() {
    assert_eq!(
        run_bootstrap_int("Phase106Test.class", "testStreamFlatMap", "()I"),
        21
    );
}

#[test]
fn test_p106_linked_list_iterator() {
    assert_eq!(
        run_bootstrap_int("Phase106Test.class", "testLinkedListIterator", "()I"),
        150
    );
}

#[test]
fn test_p106_unmodifiable_map() {
    assert_eq!(
        run_bootstrap_int("Phase106Test.class", "testUnmodifiableMap", "()I"),
        13
    );
}

#[test]
fn test_p106_string_matches() {
    assert_eq!(
        run_bootstrap_int("Phase106Test.class", "testStringMatches", "()I"),
        2
    );
}

#[test]
fn test_p106_stream_match_ops() {
    assert_eq!(
        run_bootstrap_int("Phase106Test.class", "testStreamMatchOps", "()I"),
        3
    );
}

#[test]
fn test_p106_integer_static_methods() {
    assert_eq!(
        run_bootstrap_int("Phase106Test.class", "testIntegerStaticMethods", "()I"),
        20
    );
}

#[test]
fn test_p106_string_replace_all() {
    assert_eq!(
        run_bootstrap_int("Phase106Test.class", "testStringReplaceAll", "()I"),
        19
    );
}

#[test]
fn test_p106_collectors_joining_full() {
    assert_eq!(
        run_bootstrap_int("Phase106Test.class", "testCollectorsJoiningFull", "()I"),
        9
    );
}

#[test]
fn test_p106_map_keyset() {
    assert_eq!(
        run_bootstrap_int("Phase106Test.class", "testMapKeySet", "()I"),
        3
    );
}

#[test]
fn test_p107_enum_with_methods() {
    assert_eq!(
        run_bootstrap_int("Phase107Test.class", "testEnumWithMethods", "()I"),
        2
    );
}

#[test]
fn test_p107_instanceof() {
    assert_eq!(
        run_bootstrap_int("Phase107Test.class", "testInstanceof", "()I"),
        3
    );
}

#[test]
fn test_p107_ternary_chain() {
    assert_eq!(
        run_bootstrap_int("Phase107Test.class", "testTernaryChain", "()I"),
        6
    );
}

#[test]
fn test_p107_static_initializer() {
    assert_eq!(
        run_bootstrap_int("Phase107Test.class", "testStaticInitializer", "()I"),
        13
    );
}

#[test]
fn test_p107_string_chars_distinct() {
    assert_eq!(
        run_bootstrap_int("Phase107Test.class", "testStringCharsDistinct", "()I"),
        4
    );
}

#[test]
fn test_p107_map_compute_if_absent() {
    assert_eq!(
        run_bootstrap_int("Phase107Test.class", "testMapComputeIfAbsent", "()I"),
        3
    );
}

#[test]
fn test_p107_singleton_list() {
    assert_eq!(
        run_bootstrap_int("Phase107Test.class", "testSingletonList", "()I"),
        6
    );
}

#[test]
fn test_p107_stream_count() {
    assert_eq!(
        run_bootstrap_int("Phase107Test.class", "testStreamCount", "()I"),
        47
    );
}

#[test]
fn test_p107_stringbuilder_chain() {
    assert_eq!(
        run_bootstrap_int("Phase107Test.class", "testStringBuilderChain", "()I"),
        13
    );
}

#[test]
fn test_p107_comparable() {
    assert_eq!(
        run_bootstrap_int("Phase107Test.class", "testComparable", "()I"),
        10
    );
}

#[test]
fn test_p108_substring_edge() {
    assert_eq!(
        run_bootstrap_int("Phase108Test.class", "testSubstringEdge", "()I"),
        7
    );
}

#[test]
fn test_p108_arrays_as_list_mutable() {
    assert_eq!(
        run_bootstrap_int("Phase108Test.class", "testArraysAsListMutable", "()I"),
        3
    );
}

#[test]
fn test_p108_comparator_comparing_chain() {
    assert_eq!(
        run_bootstrap_int("Phase108Test.class", "testComparatorComparingChain", "()I"),
        6
    );
}

#[test]
fn test_p108_map_foreach_sum() {
    assert_eq!(
        run_bootstrap_int("Phase108Test.class", "testMapForEachSum", "()I"),
        55
    );
}

#[test]
fn test_p108_stream_reduce_identity() {
    assert_eq!(
        run_bootstrap_int("Phase108Test.class", "testStreamReduceIdentity", "()I"),
        120
    );
}

#[test]
fn test_p108_hashmap_compute_remove() {
    assert_eq!(
        run_bootstrap_int("Phase108Test.class", "testHashMapComputeRemove", "()I"),
        0
    );
}

#[test]
fn test_p108_string_array_sort() {
    assert_eq!(
        run_bootstrap_int("Phase108Test.class", "testStringArraySort", "()I"),
        9
    );
}

#[test]
fn test_p108_collectors_counting() {
    assert_eq!(
        run_bootstrap_int("Phase108Test.class", "testCollectorsCounting", "()I"),
        5
    );
}

#[test]
fn test_p108_bifunction_and_then() {
    assert_eq!(
        run_bootstrap_int("Phase108Test.class", "testBiFunctionAndThen", "()I"),
        9
    );
}

#[test]
fn test_p108_interface_default_method() {
    assert_eq!(
        run_bootstrap_int("Phase108Test.class", "testInterfaceDefaultMethod", "()I"),
        13
    );
}

#[test]
fn test_p109_string_split_limit() {
    assert_eq!(
        run_bootstrap_int("Phase109Test.class", "testStringSplitLimit", "()I"),
        8
    );
}

#[test]
fn test_p109_abstract_class() {
    assert_eq!(
        run_bootstrap_int("Phase109Test.class", "testAbstractClass", "()I"),
        15
    );
}

#[test]
fn test_p109_map_put_all() {
    assert_eq!(
        run_bootstrap_int("Phase109Test.class", "testMapPutAll", "()I"),
        4
    );
}

#[test]
fn test_p109_grouping_by_simple() {
    assert_eq!(
        run_bootstrap_int("Phase109Test.class", "testGroupingBySimple", "()I"),
        5
    );
}

#[test]
fn test_p109_integer_parse_int_radix() {
    assert_eq!(
        run_bootstrap_int("Phase109Test.class", "testIntegerParseIntRadix", "()I"),
        245
    );
}

#[test]
fn test_p109_string_format_char() {
    assert_eq!(
        run_bootstrap_int("Phase109Test.class", "testStringFormatChar", "()I"),
        7
    );
}

#[test]
fn test_p109_list_contains() {
    assert_eq!(
        run_bootstrap_int("Phase109Test.class", "testListContains", "()I"),
        2
    );
}

#[test]
fn test_p109_stream_peek() {
    assert_eq!(
        run_bootstrap_int("Phase109Test.class", "testStreamPeek", "()I"),
        20
    );
}

#[test]
fn test_p109_nested_static_class() {
    assert_eq!(
        run_bootstrap_int("Phase109Test.class", "testNestedStaticClass", "()I"),
        50
    );
}

#[test]
fn test_p109_collections_disjoint() {
    assert_eq!(
        run_bootstrap_int("Phase109Test.class", "testCollectionsDisjoint", "()I"),
        2
    );
}

#[test]
fn test_p110_string_join() {
    assert_eq!(
        run_bootstrap_int("Phase110Test.class", "testStringJoin", "()I"),
        7
    );
}

#[test]
fn test_p110_string_join_list() {
    assert_eq!(
        run_bootstrap_int("Phase110Test.class", "testStringJoinList", "()I"),
        11
    );
}

#[test]
fn test_p110_collectors_joining() {
    assert_eq!(
        run_bootstrap_int("Phase110Test.class", "testCollectorsJoining", "()I"),
        12
    );
}

#[test]
fn test_p110_map_get_or_default() {
    assert_eq!(
        run_bootstrap_int("Phase110Test.class", "testMapGetOrDefault", "()I"),
        141
    );
}

#[test]
fn test_p110_optional_filter() {
    assert_eq!(
        run_bootstrap_int("Phase110Test.class", "testOptionalFilter", "()I"),
        19
    );
}

#[test]
fn test_p110_stream_distinct() {
    assert_eq!(
        run_bootstrap_int("Phase110Test.class", "testStreamDistinct", "()I"),
        4
    );
}

#[test]
fn test_p110_collections_sort_comparator() {
    assert_eq!(
        run_bootstrap_int("Phase110Test.class", "testCollectionsSortComparator", "()I"),
        10
    );
}

#[test]
fn test_p110_iterator_pattern() {
    assert_eq!(
        run_bootstrap_int("Phase110Test.class", "testIteratorPattern", "()I"),
        15
    );
}

#[test]
fn test_p110_map_merge() {
    assert_eq!(
        run_bootstrap_int("Phase110Test.class", "testMapMerge", "()I"),
        16
    );
}

#[test]
fn test_p110_ternary_in_stream() {
    assert_eq!(
        run_bootstrap_int("Phase110Test.class", "testTernaryInStream", "()I"),
        129
    );
}

#[test]
fn test_p111_map_put_if_absent() {
    assert_eq!(
        run_bootstrap_int("Phase111Test.class", "testMapPutIfAbsent", "()I"),
        3
    );
}

#[test]
fn test_p111_collections_reverse() {
    assert_eq!(
        run_bootstrap_int("Phase111Test.class", "testCollectionsReverse", "()I"),
        6
    );
}

#[test]
fn test_p111_stream_sorted_comparator() {
    assert_eq!(
        run_bootstrap_int("Phase111Test.class", "testStreamSortedComparator", "()I"),
        21
    );
}

#[test]
fn test_p111_int_stream_range() {
    assert_eq!(
        run_bootstrap_int("Phase111Test.class", "testIntStreamRange", "()I"),
        15
    );
}

#[test]
fn test_p111_collections_min_max() {
    assert_eq!(
        run_bootstrap_int("Phase111Test.class", "testCollectionsMinMax", "()I"),
        10
    );
}

#[test]
fn test_p111_string_contains() {
    assert_eq!(
        run_bootstrap_int("Phase111Test.class", "testStringContains", "()I"),
        2
    );
}

#[test]
fn test_p111_collectors_to_set() {
    assert_eq!(
        run_bootstrap_int("Phase111Test.class", "testCollectorsToSet", "()I"),
        3
    );
}

#[test]
fn test_p111_stream_matchers() {
    assert_eq!(
        run_bootstrap_int("Phase111Test.class", "testStreamMatchers", "()I"),
        3
    );
}

#[test]
fn test_p111_varargs() {
    assert_eq!(
        run_bootstrap_int("Phase111Test.class", "testVarargs", "()I"),
        36
    );
}

#[test]
fn test_p111_string_builder_chaining() {
    assert_eq!(
        run_bootstrap_int("Phase111Test.class", "testStringBuilderChaining", "()I"),
        13
    );
}

#[test]
fn test_p112_map_contains_value() {
    assert_eq!(
        run_bootstrap_int("Phase112Test.class", "testMapContainsValue", "()I"),
        2
    );
}

#[test]
fn test_p112_list_sub_list() {
    assert_eq!(
        run_bootstrap_int("Phase112Test.class", "testListSubList", "()I"),
        90
    );
}

#[test]
fn test_p112_collections_frequency() {
    assert_eq!(
        run_bootstrap_int("Phase112Test.class", "testCollectionsFrequency", "()I"),
        4
    );
}

#[test]
fn test_p112_collect_to_map() {
    assert_eq!(
        run_bootstrap_int("Phase112Test.class", "testCollectToMap", "()I"),
        17
    );
}

#[test]
fn test_p112_integer_compare() {
    assert_eq!(
        run_bootstrap_int("Phase112Test.class", "testIntegerCompare", "()I"),
        0
    );
}

#[test]
fn test_p112_string_chars_stream() {
    assert_eq!(
        run_bootstrap_int("Phase112Test.class", "testStringCharsStream", "()I"),
        3
    );
}

#[test]
fn test_p112_collectors_counting() {
    assert_eq!(
        run_bootstrap_int("Phase112Test.class", "testCollectorsCounting", "()I"),
        4
    );
}

#[test]
fn test_p112_array_deque_as_stack() {
    assert_eq!(
        run_bootstrap_int("Phase112Test.class", "testArrayDequeAsStack", "()I"),
        6
    );
}

#[test]
fn test_p112_map_key_set_iteration() {
    assert_eq!(
        run_bootstrap_int("Phase112Test.class", "testMapKeySetIteration", "()I"),
        60
    );
}

#[test]
fn test_p112_stream_limit() {
    assert_eq!(
        run_bootstrap_int("Phase112Test.class", "testStreamLimit", "()I"),
        15
    );
}

#[test]
fn test_p113_linked_hash_map() {
    assert_eq!(
        run_bootstrap_int("Phase113Test.class", "testLinkedHashMap", "()I"),
        6
    );
}

#[test]
fn test_p113_collections_swap() {
    assert_eq!(
        run_bootstrap_int("Phase113Test.class", "testCollectionsSwap", "()I"),
        60
    );
}

#[test]
fn test_p113_stream_generate() {
    assert_eq!(
        run_bootstrap_int("Phase113Test.class", "testStreamGenerate", "()I"),
        3
    );
}

#[test]
fn test_p113_collectors_averaging_int() {
    assert_eq!(
        run_bootstrap_int("Phase113Test.class", "testCollectorsAveragingInt", "()I"),
        3
    );
}

#[test]
fn test_p113_map_compute_if_absent() {
    assert_eq!(
        run_bootstrap_int("Phase113Test.class", "testMapComputeIfAbsent", "()I"),
        3
    );
}

#[test]
fn test_p113_integer_bit_count() {
    assert_eq!(
        run_bootstrap_int("Phase113Test.class", "testIntegerBitCount", "()I"),
        11
    );
}

#[test]
fn test_p113_string_format_multiple() {
    assert_eq!(
        run_bootstrap_int("Phase113Test.class", "testStringFormatMultiple", "()I"),
        16
    );
}

#[test]
fn test_p113_stream_empty() {
    assert_eq!(
        run_bootstrap_int("Phase113Test.class", "testStreamEmpty", "()I"),
        0
    );
}

#[test]
fn test_p113_list_set() {
    assert_eq!(
        run_bootstrap_int("Phase113Test.class", "testListSet", "()I"),
        111
    );
}

#[test]
fn test_p113_grouping_by_downstream() {
    assert_eq!(
        run_bootstrap_int("Phase113Test.class", "testGroupingByDownstream", "()I"),
        6
    );
}

#[test]
fn test_p114_stack_class() {
    assert_eq!(
        run_bootstrap_int("Phase114Test.class", "testStackClass", "()I"),
        62
    );
}

#[test]
fn test_p114_collections_shuffle() {
    assert_eq!(
        run_bootstrap_int("Phase114Test.class", "testCollectionsShuffle", "()I"),
        15
    );
}

#[test]
fn test_p114_collectors_summing_int() {
    assert_eq!(
        run_bootstrap_int("Phase114Test.class", "testCollectorsSummingInt", "()I"),
        10
    );
}

#[test]
fn test_p114_integer_leading_zeros() {
    assert_eq!(
        run_bootstrap_int("Phase114Test.class", "testIntegerLeadingZeros", "()I"),
        31
    );
}

#[test]
fn test_p114_map_values_iteration() {
    assert_eq!(
        run_bootstrap_int("Phase114Test.class", "testMapValuesIteration", "()I"),
        30
    );
}

#[test]
fn test_p114_instance_of() {
    assert_eq!(
        run_bootstrap_int("Phase114Test.class", "testInstanceOf", "()I"),
        3
    );
}

#[test]
fn test_p114_string_substring_edge() {
    assert_eq!(
        run_bootstrap_int("Phase114Test.class", "testStringSubstringEdge", "()I"),
        11
    );
}

#[test]
fn test_p114_stream_flat_map() {
    assert_eq!(
        run_bootstrap_int("Phase114Test.class", "testStreamFlatMap", "()I"),
        45
    );
}

#[test]
fn test_p114_enum_values() {
    assert_eq!(
        run_bootstrap_int("Phase114Test.class", "testEnumValues", "()I"),
        6
    );
}

#[test]
fn test_p114_integer_max_min() {
    assert_eq!(
        run_bootstrap_int("Phase114Test.class", "testIntegerMaxMin", "()I"),
        30
    );
}

#[test]
fn test_p115_string_chars_to_list() {
    assert_eq!(
        run_bootstrap_int("Phase115Test.class", "testStringCharsToList", "()I"),
        196
    );
}

#[test]
fn test_p115_optional_if_present() {
    assert_eq!(
        run_bootstrap_int("Phase115Test.class", "testOptionalIfPresent", "()I"),
        42
    );
}

#[test]
fn test_p115_singleton_list() {
    assert_eq!(
        run_bootstrap_int("Phase115Test.class", "testSingletonList", "()I"),
        6
    );
}

#[test]
fn test_p115_stream_reduce_identity() {
    assert_eq!(
        run_bootstrap_int("Phase115Test.class", "testStreamReduceIdentity", "()I"),
        120
    );
}

#[test]
fn test_p115_map_entry_set_for_each() {
    assert_eq!(
        run_bootstrap_int("Phase115Test.class", "testMapEntrySetForEach", "()I"),
        60
    );
}

#[test]
fn test_p115_integer_reverse() {
    assert_eq!(
        run_bootstrap_int("Phase115Test.class", "testIntegerReverse", "()I"),
        1
    );
}

#[test]
fn test_p115_collectors_to_unmodifiable_map() {
    assert_eq!(
        run_bootstrap_int(
            "Phase115Test.class",
            "testCollectorsToUnmodifiableMap",
            "()I"
        ),
        16
    );
}

#[test]
fn test_p115_long_compare() {
    assert_eq!(
        run_bootstrap_int("Phase115Test.class", "testLongCompare", "()I"),
        0
    );
}

#[test]
fn test_p115_arrays_stream() {
    assert_eq!(
        run_bootstrap_int("Phase115Test.class", "testArraysStream", "()I"),
        50
    );
}

#[test]
fn test_p115_nested_generics() {
    assert_eq!(
        run_bootstrap_int("Phase115Test.class", "testNestedGenerics", "()I"),
        21
    );
}

#[test]
fn test_p116_tree_set() {
    assert_eq!(
        run_bootstrap_int("Phase116Test.class", "testTreeSet", "()I"),
        15
    );
}

#[test]
fn test_p116_stream_to_array() {
    assert_eq!(
        run_bootstrap_int("Phase116Test.class", "testStreamToArray", "()I"),
        3
    );
}

#[test]
fn test_p116_deque_as_queue() {
    assert_eq!(
        run_bootstrap_int("Phase116Test.class", "testDequeAsQueue", "()I"),
        6
    );
}

#[test]
fn test_p116_map_for_each_accumulate() {
    assert_eq!(
        run_bootstrap_int("Phase116Test.class", "testMapForEachAccumulate", "()I"),
        24
    );
}

#[test]
fn test_p116_string_value_of() {
    assert_eq!(
        run_bootstrap_int("Phase116Test.class", "testStringValueOf", "()I"),
        10
    );
}

#[test]
fn test_p116_int_stream_range_closed() {
    assert_eq!(
        run_bootstrap_int("Phase116Test.class", "testIntStreamRangeClosed", "()I"),
        15
    );
}

#[test]
fn test_p116_partitioning_by_downstream() {
    assert_eq!(
        run_bootstrap_int("Phase116Test.class", "testPartitioningByDownstream", "()I"),
        6
    );
}

#[test]
fn test_p116_optional_or_else_get() {
    assert_eq!(
        run_bootstrap_int("Phase116Test.class", "testOptionalOrElseGet", "()I"),
        52
    );
}

#[test]
fn test_p116_stream_peek_count() {
    assert_eq!(
        run_bootstrap_int("Phase116Test.class", "testStreamPeekCount", "()I"),
        17
    );
}

#[test]
fn test_p116_comparable() {
    assert_eq!(
        run_bootstrap_int("Phase116Test.class", "testComparable", "()I"),
        140
    );
}

// Phase 117: Map.remove(k,v), Collections.emptyList, Stream.concat, Set.forEach,
//            Map.replace, Integer.sum, IntStream.mapToObj, Collections.unmodifiableSet,
//            Optional.map, 2D arrays
#[test]
fn test_p117_map_remove_key_value() {
    assert_eq!(
        run_bootstrap_int("Phase117Test.class", "testMapRemoveKeyValue", "()I"),
        3
    );
}
#[test]
fn test_p117_collections_empty_list() {
    assert_eq!(
        run_bootstrap_int("Phase117Test.class", "testCollectionsEmptyList", "()I"),
        10
    );
}
#[test]
fn test_p117_stream_concat() {
    assert_eq!(
        run_bootstrap_int("Phase117Test.class", "testStreamConcat", "()I"),
        21
    );
}
#[test]
fn test_p117_set_for_each() {
    assert_eq!(
        run_bootstrap_int("Phase117Test.class", "testSetForEach", "()I"),
        15
    );
}
#[test]
fn test_p117_map_replace() {
    assert_eq!(
        run_bootstrap_int("Phase117Test.class", "testMapReplace", "()I"),
        100
    );
}
#[test]
fn test_p117_integer_sum() {
    assert_eq!(
        run_bootstrap_int("Phase117Test.class", "testIntegerSum", "()I"),
        300
    );
}
#[test]
fn test_p117_int_stream_map_to_obj() {
    assert_eq!(
        run_bootstrap_int("Phase117Test.class", "testIntStreamMapToObj", "()I"),
        5
    );
}
#[test]
fn test_p117_unmodifiable_set() {
    assert_eq!(
        run_bootstrap_int("Phase117Test.class", "testUnmodifiableSet", "()I"),
        13
    );
}
#[test]
fn test_p117_optional_map() {
    assert_eq!(
        run_bootstrap_int("Phase117Test.class", "testOptionalMap", "()I"),
        4
    );
}
#[test]
fn test_p117_array_of_arrays() {
    assert_eq!(
        run_bootstrap_int("Phase117Test.class", "testArrayOfArrays", "()I"),
        45
    );
}

#[test]
fn test_box_primitive_slot() {
    let mut heap = duke_gc::Heap::new();

    let cases = vec![
        (
            duke_runtime::Slot::Int(42),
            "java/lang/Integer",
            duke_runtime::Slot::Int(42),
        ),
        (
            duke_runtime::Slot::Long(999),
            "java/lang/Long",
            duke_runtime::Slot::Long(999),
        ),
        (
            duke_runtime::Slot::Float(1.23),
            "java/lang/Float",
            duke_runtime::Slot::Float(1.23),
        ),
        (
            duke_runtime::Slot::Double(4.56),
            "java/lang/Double",
            duke_runtime::Slot::Double(4.56),
        ),
    ];

    for (input, expected_class, expected_field) in cases {
        let result = crate::box_primitive_slot(input, &mut heap);

        let duke_runtime::Slot::Reference(Some(obj_ref)) = result else {
            panic!("Expected boxed primitive to be a Reference(Some(r))")
        };

        let obj = heap.get(obj_ref).expect("Object must exist in heap");
        assert_eq!(obj.class_name, expected_class);
        assert_eq!(obj.fields[0], expected_field);
    }

    let none_ref = duke_runtime::Slot::Reference(None);
    let result = crate::box_primitive_slot(none_ref, &mut heap);
    assert_eq!(
        result, none_ref,
        "Expected pass-through for other slot types"
    );
}

/// Minimal [`CallbackOps`] stub whose `inspect_class` returns a fixed
/// [`ReflectedClassInfo`], for exercising the access-flag reflection natives.
struct FixedClassInfoOps {
    info: ReflectedClassInfo,
}

impl CallbackOps for FixedClassInfoOps {
    fn invoke(
        &mut self,
        _heap: &mut duke_gc::Heap,
        _output: &mut dyn Write,
        _class: &str,
        _method: &str,
        _descriptor: &str,
        _args: Vec<Slot>,
    ) -> Result<Option<Slot>> {
        Ok(None)
    }

    fn ensure_loaded(&mut self, _class: &str) -> Result<()> {
        Ok(())
    }

    fn inspect_class(&mut self, _class: &str) -> Result<ReflectedClassInfo> {
        Ok(self.info.clone())
    }
}

fn class_info_with_flags(internal_name: &str, access_flags: u16) -> ReflectedClassInfo {
    ReflectedClassInfo {
        internal_name: internal_name.to_string(),
        binary_name: internal_name.replace('/', "."),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        access_flags,
        annotations: Vec::new(),
    }
}

#[test]
fn native_class_get_modifiers_reports_real_flags_and_strips_super() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let class_ref = allocate_class_object(&mut heap, "com/example/Widget").unwrap();

    // ACC_PUBLIC | ACC_SUPER | ACC_FINAL (0x0031) — as javac emits for a final
    // class. getModifiers must strip ACC_SUPER (0x0020) and report 0x0011.
    let mut ops = FixedClassInfoOps {
        info: class_info_with_flags("com/example/Widget", 0x0031),
    };
    let modifiers = native_class_get_modifiers(
        &[Slot::Reference(Some(class_ref))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
        &mut ops,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        modifiers,
        Slot::Int(0x0011),
        "PUBLIC|FINAL, ACC_SUPER stripped"
    );
}

#[test]
fn native_class_is_interface_reads_real_acc_interface_bit() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let class_ref = allocate_class_object(&mut heap, "com/example/Service").unwrap();

    // ACC_PUBLIC | ACC_INTERFACE | ACC_ABSTRACT (0x0601).
    let mut ops = FixedClassInfoOps {
        info: class_info_with_flags("com/example/Service", 0x0601),
    };
    let is_iface = native_class_is_interface(
        &[Slot::Reference(Some(class_ref))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
        &mut ops,
    )
    .unwrap()
    .unwrap();
    assert_eq!(is_iface, Slot::Int(1), "interface flag reported");

    let modifiers = native_class_get_modifiers(
        &[Slot::Reference(Some(class_ref))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
        &mut ops,
    )
    .unwrap()
    .unwrap();
    assert_eq!(modifiers, Slot::Int(0x0601), "PUBLIC|INTERFACE|ABSTRACT");
}

#[test]
fn native_class_is_interface_false_for_concrete_class() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let class_ref = allocate_class_object(&mut heap, "com/example/Widget").unwrap();
    let mut ops = FixedClassInfoOps {
        info: class_info_with_flags("com/example/Widget", 0x0021),
    };
    let is_iface = native_class_is_interface(
        &[Slot::Reference(Some(class_ref))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
        &mut ops,
    )
    .unwrap()
    .unwrap();
    assert_eq!(is_iface, Slot::Int(0));
}

#[test]
fn native_reflect_field_get_modifiers_reports_transient_and_final() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();

    // A `private final transient int cache;` field (0x0002|0x0010|0x0080 = 0x0092).
    let field_ref = allocate_reflection_member_object(
        &mut heap,
        "java/lang/reflect/Field",
        "com/example/Widget",
        "cache",
        "I",
        false, // is_public
        false, // is_static
    )
    .unwrap();

    let mut info = class_info_with_flags("com/example/Widget", 0x0021);
    info.fields = vec![ReflectedFieldInfo {
        name: "cache".to_string(),
        descriptor: "I".to_string(),
        is_public: false,
        is_static: false,
        access_flags: 0x0092,
        annotations: Vec::new(),
    }];
    let mut ops = FixedClassInfoOps { info };

    let modifiers = native_reflect_field_get_modifiers(
        &[Slot::Reference(Some(field_ref))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
        &mut ops,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        modifiers,
        Slot::Int(0x0092),
        "PRIVATE|FINAL|TRANSIENT surfaced (was PUBLIC/STATIC-only before)"
    );
}

#[test]
fn native_class_get_module_returns_interned_unnamed_module() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let class_a = allocate_class_object(&mut heap, "com/example/A").unwrap();
    let class_b = allocate_class_object(&mut heap, "com/example/B").unwrap();

    let module_a = native_class_get_module(
        &[Slot::Reference(Some(class_a))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .unwrap()
    .unwrap();
    let module_b = native_class_get_module(
        &[Slot::Reference(Some(class_b))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .unwrap()
    .unwrap();

    // Every class shares the one unnamed-module instance (identity matters to
    // ServiceLoader's module comparisons).
    assert_eq!(module_a, module_b, "getModule() interns a single instance");

    let Slot::Reference(Some(module_ref)) = module_a else {
        panic!("expected a Module reference");
    };
    assert_eq!(heap.get(module_ref).unwrap().class_name, "java/lang/Module");

    // isNamed() == false, getName() == null, canUse(..) == true.
    let is_named = native_module_is_named(
        &[module_a],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(is_named, Slot::Int(0), "unnamed module: isNamed() == false");

    let name = native_module_get_name(
        &[module_a],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        name,
        Slot::Reference(None),
        "unnamed module: getName() == null"
    );

    let can_use = native_module_can_use(
        &[module_a, Slot::Reference(Some(class_b))],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(can_use, Slot::Int(1), "unnamed module canUse(..) == true");
}

// ---------------------------------------------------------------------------
// Linkage-error lane: catchable NoClassDefFoundError + JVMS 5.5 erroneous-class
// semantics. Fixtures: tests/fixtures/{NoClassDefFoundCatch,LinkageErrorProbes}.java
// (their `Missing` / `NoClassDefFoundCatchHelper` helper classes are compiled but
// intentionally not committed, so they are unresolvable at run time).
// ---------------------------------------------------------------------------

/// Run a no-arg `static int` probe on the `LinkageErrorProbes` fixture, returning
/// the raw execution result so tests can inspect uncaught errors.
fn run_linkage_probe_result(method_name: &str) -> Result<Option<Slot>> {
    let ctx = load_class_context("LinkageErrorProbes.class");
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
        "()I",
        &[],
    )
}

#[test]
fn linkage_invokestatic_missing_class_is_catchable_ncdfe_with_message() {
    // invokestatic against an unresolvable class throws NoClassDefFoundError
    // whose message names the missing class (probe returns 1 only if both hold).
    assert_eq!(
        run_bootstrap_int_completion("LinkageErrorProbes.class", "probeInvokestatic", "()I"),
        1
    );
}

// NOTE: there is deliberately no `new`-opcode NCDFE test. Unlike the other
// resolution sites, `new` on an unmodelled class historically "limps" (allocates
// a zero-field object) rather than failing fatally, and real-jar boot progress
// depends on that; see the comment at the New handler in execution.rs.

#[test]
fn linkage_getstatic_missing_class_is_catchable_ncdfe() {
    assert_eq!(
        run_bootstrap_int_completion("LinkageErrorProbes.class", "probeGetstatic", "()I"),
        1
    );
}

#[test]
fn linkage_putstatic_missing_class_is_catchable_ncdfe() {
    assert_eq!(
        run_bootstrap_int_completion("LinkageErrorProbes.class", "probePutstatic", "()I"),
        1
    );
}

#[test]
fn linkage_ncdfe_caught_as_linkage_error_superclass_falls_through() {
    // commons-logging pattern: catch (LinkageError) around a missing-class use
    // and fall through to an alternative.
    assert_eq!(
        run_bootstrap_int_completion(
            "LinkageErrorProbes.class",
            "probeLinkageErrorFallback",
            "()I"
        ),
        1
    );
}

#[test]
fn linkage_ncdfe_caught_as_throwable_root() {
    assert_eq!(
        run_bootstrap_int_completion("LinkageErrorProbes.class", "probeThrowableCatch", "()I"),
        1
    );
}

#[test]
fn linkage_uncaught_ncdfe_surfaces_as_java_exception_of_correct_type() {
    // Directly assert the propagated Error variant / class name.
    match run_linkage_probe_result("uncaughtInvokestatic") {
        Err(Error::JavaException { class_name }) => {
            assert_eq!(class_name, "java/lang/NoClassDefFoundError");
        }
        other => panic!("expected uncaught NoClassDefFoundError JavaException, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Honest-dispatch lane: catchable NoSuchMethodError (JVMS 5.4.3.3). Making
// lenient method dispatch honest means a call to a method on an unresolvable
// class now throws a *catchable* NoSuchMethodError instead of the former silent
// soft-fail (which popped args + `this` and continued without pushing a return
// value, corrupting the operand stack).
//
// Fixture: tests/fixtures/NoSuchMethodProbes.java — its `MissingMethods` helper
// is compiled but intentionally NOT committed, so it is unresolvable at run time.
// `new MissingMethods()` emits `invokespecial MissingMethods.<init>()V` against
// the unloadable class, which the interpreter reports as a NoSuchMethodError.
// ---------------------------------------------------------------------------

/// Run a no-arg `static int` probe on the `NoSuchMethodProbes` fixture, returning
/// the raw execution result so tests can inspect uncaught errors.
fn run_no_such_method_probe_result(method_name: &str) -> Result<Option<Slot>> {
    let ctx = load_class_context("NoSuchMethodProbes.class");
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
        "()I",
        &[],
    )
}

#[test]
fn nsme_missing_method_is_catchable_with_owner_method_descriptor_message() {
    // probe returns 1 iff a catchable NoSuchMethodError is caught AND its detail
    // message names the owner class, the method, and the descriptor.
    assert_eq!(
        run_bootstrap_int_completion(
            "NoSuchMethodProbes.class",
            "probeCatchNoSuchMethodError",
            "()I"
        ),
        1
    );
}

#[test]
fn nsme_catchable_as_incompatible_class_change_error() {
    // NoSuchMethodError extends IncompatibleClassChangeError (JVMS 5.4.3.3).
    assert_eq!(
        run_bootstrap_int_completion(
            "NoSuchMethodProbes.class",
            "probeCatchIncompatibleClassChangeError",
            "()I"
        ),
        1
    );
}

#[test]
fn nsme_catchable_as_linkage_error_superclass() {
    // IncompatibleClassChangeError extends LinkageError.
    assert_eq!(
        run_bootstrap_int_completion("NoSuchMethodProbes.class", "probeCatchLinkageError", "()I"),
        1
    );
}

#[test]
fn nsme_catchable_as_throwable_root() {
    assert_eq!(
        run_bootstrap_int_completion("NoSuchMethodProbes.class", "probeCatchThrowable", "()I"),
        1
    );
}

#[test]
fn nsme_uncaught_surfaces_as_java_exception_of_correct_type() {
    match run_no_such_method_probe_result("uncaughtNoSuchMethod") {
        Err(Error::JavaException { class_name }) => {
            assert_eq!(class_name, "java/lang/NoSuchMethodError");
        }
        other => panic!("expected uncaught NoSuchMethodError JavaException, got {other:?}"),
    }
}

#[test]
fn clinit_throwing_error_propagates_unwrapped() {
    // A <clinit> that throws an Error surfaces that Error as-is (not wrapped in
    // ExceptionInInitializerError): probe returns 1 from catch (Error).
    assert_eq!(
        run_bootstrap_int_completion(
            "LinkageErrorProbes.class",
            "probeClinitErrorUnwrapped",
            "()I"
        ),
        1
    );
}

#[test]
fn clinit_throwing_exception_is_wrapped_in_exception_in_initializer_error() {
    assert_eq!(
        run_bootstrap_int_completion(
            "LinkageErrorProbes.class",
            "probeClinitRuntimeWrapped",
            "()I"
        ),
        1
    );
}

#[test]
fn erroneous_class_second_use_throws_ncdfe() {
    // JVMS 5.5: after a failed <clinit> the class is erroneous; a later use
    // raises NoClassDefFoundError instead of re-running the initializer.
    assert_eq!(
        run_bootstrap_int_completion("LinkageErrorProbes.class", "probeErroneousSecondUse", "()I"),
        1
    );
}

#[test]
fn compiled_fixture_catches_no_class_def_found_error() {
    // Compiled Java fixture with `static int probe()` returning 1 from a
    // catch (NoClassDefFoundError e) block over an unresolvable helper class.
    assert_eq!(
        run_bootstrap_int_completion("NoClassDefFoundCatch.class", "probe", "()I"),
        1
    );
}

#[test]
fn registry_tracks_erroneous_class_state() {
    let mut registry = ClassRegistry::new();
    assert!(!registry.is_erroneous("com/example/Boom"));
    registry.mark_initialized("com/example/Boom");
    registry.mark_erroneous("com/example/Boom");
    assert!(registry.is_erroneous("com/example/Boom"));
    // mark_erroneous clears the initialized flag for a single coherent state.
    assert!(!registry.is_initialized("com/example/Boom"));
}

#[test]
fn linkage_error_hierarchy_is_registered_for_catch_matching() {
    // NoClassDefFoundError must resolve up through LinkageError -> Error ->
    // Throwable so catch clauses on any of those match it.
    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = fixtures_loader();
    for target in [
        "java/lang/LinkageError",
        "java/lang/Error",
        "java/lang/Throwable",
    ] {
        assert!(
            is_assignable_from(
                &mut registry,
                &loader,
                "java/lang/NoClassDefFoundError",
                target,
                None,
            ),
            "NoClassDefFoundError should be assignable to {target}"
        );
    }
    assert!(is_assignable_from(
        &mut registry,
        &loader,
        "java/lang/ExceptionInInitializerError",
        "java/lang/LinkageError",
        None,
    ));
}

// java.lang.invoke / SharedSecrets foundation natives
// ---------------------------------------------------------------------------
// These walk the real `FileOutputStream.<clinit>` chain under `DUKE_REAL_JDK=1`:
// `Unsafe.ensureClassInitialized` forces `FileDescriptor.<clinit>`, whose real
// bytecode calls `initIDs`/`getHandle`/`getAppend`. They are dormant while
// `FileOutputStream` stays on `KEEP_SYNTHETIC`, so these direct-dispatch tests
// pin their contracts against bit-rot.

/// `Unsafe.ensureClassInitialized(Class)` must drive the argument class (arg 1,
/// after the ignored `Unsafe` receiver) through `ensure_class_initialized`.
#[test]
fn native_unsafe_ensure_class_initialized_forces_arg_class() {
    struct RecordingOps {
        initialized: Vec<String>,
    }

    impl CallbackOps for RecordingOps {
        fn invoke(
            &mut self,
            _heap: &mut duke_gc::Heap,
            _output: &mut dyn Write,
            _class: &str,
            _method: &str,
            _descriptor: &str,
            _args: Vec<Slot>,
        ) -> Result<Option<Slot>> {
            Ok(None)
        }

        fn ensure_loaded(&mut self, _class: &str) -> Result<()> {
            Ok(())
        }

        fn inspect_class(&mut self, _class: &str) -> Result<ReflectedClassInfo> {
            unreachable!("ensureClassInitialized must not inspect")
        }

        fn ensure_class_initialized(
            &mut self,
            _heap: &mut duke_gc::Heap,
            _output: &mut dyn Write,
            class: &str,
        ) -> Result<()> {
            self.initialized.push(class.to_string());
            Ok(())
        }
    }

    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let unsafe_ref = heap.allocate("jdk/internal/misc/Unsafe".to_string(), 0);
    let class_ref = allocate_class_object(&mut heap, "java/io/FileDescriptor").unwrap();
    let mut ops = RecordingOps {
        initialized: Vec::new(),
    };

    let ret = native_unsafe_ensure_class_initialized(
        &[
            Slot::Reference(Some(unsafe_ref)),
            Slot::Reference(Some(class_ref)),
        ],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
        &mut ops,
    )
    .unwrap();

    assert_eq!(ret, None, "ensureClassInitialized returns void");
    assert_eq!(
        ops.initialized,
        vec!["java/io/FileDescriptor".to_string()],
        "the argument class must be forced through <clinit>"
    );
}

/// `FileDescriptor`/`FileOutputStream` `initIDs()V` is an honest no-op under
/// Duke's positional field model (nothing to cache).
#[test]
fn native_io_init_ids_is_noop() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let ret =
        native_io_init_ids_noop(&[], &mut heap, &mut sink, &mut NativeControl::default()).unwrap();
    assert_eq!(ret, None, "initIDs returns void and caches nothing");
}

/// Unix `FileDescriptor.getHandle(int)` returns -1 unconditionally (handles are
/// a Windows-only concept).
#[test]
fn native_file_descriptor_get_handle_is_minus_one_on_unix() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let ret = native_file_descriptor_get_handle(
        &[Slot::Int(0)],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(ret, Slot::Long(-1), "getHandle == -1 on unix");
}

/// `FileDescriptor.getAppend(int)` is false for the standard descriptors built
/// in `<clinit>` (none opened in append mode).
#[test]
fn native_file_descriptor_get_append_is_false() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let ret = native_file_descriptor_get_append(
        &[Slot::Int(1)],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(ret, Slot::Int(0), "getAppend == false");
}

// ── Stage-b file-layer floor: `FileOutputStream.writeBytes` / `initIDs` JLA seed ──
//
// These natives are dormant while `FileOutputStream` stays on `KEEP_SYNTHETIC`
// (real-layout `FileOutputStream` is only loaded when the allowlist drops it). The
// direct-dispatch tests below pin their contracts against bit-rot, mirroring the
// `initIDs`/`getHandle`/`getAppend` tests above.

/// A `CallbackOps` mock that models a real-layout `FileOutputStream -> fd
/// FileDescriptor -> fd int` object graph and records static-field writes.
struct FileStreamOps {
    /// `this.fd` (a `FileDescriptor` reference), returned for `FileOutputStream.fd`.
    fd_descriptor_ref: u64,
    /// The `int` OS descriptor, returned for `FileDescriptor.fd`.
    fd_value: i32,
    /// Current value of `SharedSecrets.javaLangAccess` (starts null).
    java_lang_access: Slot,
    /// Records `(class, field, value)` static writes.
    static_writes: Vec<(String, String, Slot)>,
    /// Records `ensure_loaded` calls.
    loaded: Vec<String>,
}

impl CallbackOps for FileStreamOps {
    fn invoke(
        &mut self,
        _heap: &mut duke_gc::Heap,
        _output: &mut dyn Write,
        _class: &str,
        _method: &str,
        _descriptor: &str,
        _args: Vec<Slot>,
    ) -> Result<Option<Slot>> {
        Ok(None)
    }

    fn ensure_loaded(&mut self, class: &str) -> Result<()> {
        self.loaded.push(class.to_string());
        Ok(())
    }

    fn inspect_class(&mut self, _class: &str) -> Result<ReflectedClassInfo> {
        unreachable!("file-stream natives must not inspect classes")
    }

    fn read_instance_field(
        &mut self,
        _heap: &duke_gc::Heap,
        object_ref: u64,
        declaring_class: &str,
        field_name: &str,
    ) -> Result<Slot> {
        match (declaring_class, field_name) {
            ("java/io/FileOutputStream", "fd") => Ok(Slot::Reference(Some(self.fd_descriptor_ref))),
            ("java/io/FileDescriptor", "fd") => {
                assert_eq!(
                    object_ref, self.fd_descriptor_ref,
                    "fd read on the descriptor"
                );
                Ok(Slot::Int(self.fd_value))
            }
            other => unreachable!("unexpected instance-field read: {other:?}"),
        }
    }

    fn read_static_field(&mut self, class: &str, field_name: &str) -> Result<Slot> {
        assert_eq!(
            (class, field_name),
            ("jdk/internal/access/SharedSecrets", "javaLangAccess")
        );
        Ok(self.java_lang_access)
    }

    fn write_static_field(&mut self, class: &str, field_name: &str, value: Slot) -> Result<()> {
        self.static_writes
            .push((class.to_string(), field_name.to_string(), value));
        if class == "jdk/internal/access/SharedSecrets" && field_name == "javaLangAccess" {
            self.java_lang_access = value;
        }
        Ok(())
    }
}

/// `FileOutputStream.writeBytes([BIIZ)` resolves `this.fd.fd`, and for the stdout
/// descriptor (`fd == 1`) writes the `[offset, offset+len)` slice of the byte
/// array to the interpreter output sink.
#[test]
fn native_real_file_output_stream_write_bytes_routes_stdout() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();

    // byte[] = { 'H','E','L','L','O' }; write the middle three (offset 1, len 3).
    let array_ref = heap.allocate("[B".to_string(), 5);
    for (i, b) in (*b"HELLO").into_iter().enumerate() {
        heap.get_mut(array_ref).unwrap().fields[i] = Slot::Int(i32::from(b));
    }
    let this_ref = heap.allocate("java/io/FileOutputStream".to_string(), 1);
    let fd_ref = heap.allocate("java/io/FileDescriptor".to_string(), 3);

    let mut ops = FileStreamOps {
        fd_descriptor_ref: fd_ref,
        fd_value: 1,
        java_lang_access: Slot::Reference(None),
        static_writes: Vec::new(),
        loaded: Vec::new(),
    };

    let ret = native_real_file_output_stream_write_bytes(
        &[
            Slot::Reference(Some(this_ref)),
            Slot::Reference(Some(array_ref)),
            Slot::Int(1),
            Slot::Int(3),
            Slot::Int(0),
        ],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
        &mut ops,
    )
    .unwrap();

    assert_eq!(ret, None, "writeBytes returns void");
    assert_eq!(sink, b"ELL", "stdout descriptor writes the requested slice");
}

/// Writing to a descriptor that is neither stdout (1) nor stderr (2) — a
/// path-backed `open0` fd, not yet modelled — surfaces an honest `IOException`
/// rather than silently succeeding.
#[test]
fn native_real_file_output_stream_write_bytes_unsupported_fd_errors() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();

    let array_ref = heap.allocate("[B".to_string(), 1);
    heap.get_mut(array_ref).unwrap().fields[0] = Slot::Int(42);
    let this_ref = heap.allocate("java/io/FileOutputStream".to_string(), 1);
    let fd_ref = heap.allocate("java/io/FileDescriptor".to_string(), 3);

    let mut ops = FileStreamOps {
        fd_descriptor_ref: fd_ref,
        fd_value: 99,
        java_lang_access: Slot::Reference(None),
        static_writes: Vec::new(),
        loaded: Vec::new(),
    };

    let err = native_real_file_output_stream_write_bytes(
        &[
            Slot::Reference(Some(this_ref)),
            Slot::Reference(Some(array_ref)),
            Slot::Int(0),
            Slot::Int(1),
            Slot::Int(0),
        ],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
        &mut ops,
    )
    .unwrap_err();

    match err {
        Error::JavaException { class_name } => {
            assert_eq!(class_name, "java/io/IOException");
        }
        other => panic!("expected IOException, got {other:?}"),
    }
    assert!(
        sink.is_empty(),
        "no bytes written to an unsupported descriptor"
    );
}

/// `FileOutputStream.initIDs()` installs a non-null placeholder
/// `SharedSecrets.javaLangAccess` (satisfying the `Blocker.<clinit>` boot check)
/// when the field is null, and loads `SharedSecrets` first.
#[test]
fn native_file_output_stream_init_ids_installs_java_lang_access() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();

    let mut ops = FileStreamOps {
        fd_descriptor_ref: 0,
        fd_value: 0,
        java_lang_access: Slot::Reference(None),
        static_writes: Vec::new(),
        loaded: Vec::new(),
    };

    let ret = native_file_output_stream_init_ids(
        &[],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
        &mut ops,
    )
    .unwrap();

    assert_eq!(ret, None, "initIDs returns void");
    assert!(
        ops.loaded
            .contains(&"jdk/internal/access/SharedSecrets".to_string()),
        "SharedSecrets must be ensured loaded before its static is written"
    );
    assert_eq!(ops.static_writes.len(), 1, "exactly one static write");
    let (class, field, value) = &ops.static_writes[0];
    assert_eq!(class, "jdk/internal/access/SharedSecrets");
    assert_eq!(field, "javaLangAccess");
    assert!(
        matches!(value, Slot::Reference(Some(_))),
        "a non-null JavaLangAccess placeholder is installed"
    );
}

/// `initIDs()` is idempotent: when `SharedSecrets.javaLangAccess` is already
/// installed, it does not overwrite it (no static write).
#[test]
fn native_file_output_stream_init_ids_is_idempotent() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();
    let existing = heap.allocate("jdk/internal/access/JavaLangAccess".to_string(), 0);

    let mut ops = FileStreamOps {
        fd_descriptor_ref: 0,
        fd_value: 0,
        java_lang_access: Slot::Reference(Some(existing)),
        static_writes: Vec::new(),
        loaded: Vec::new(),
    };

    native_file_output_stream_init_ids(
        &[],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
        &mut ops,
    )
    .unwrap();

    assert!(
        ops.static_writes.is_empty(),
        "an already-installed JavaLangAccess must not be overwritten"
    );
}

/// Regression for the false-negative `instanceof`/`checkcast` that walled the
/// commons-logging ladder: `LogFactoryImpl.createLogFromClass` evaluates
/// `newLogger instanceof org/apache/commons/logging/Log` on a reflectively-built
/// `Jdk14Logger` whose `ClassContext` records its interface as a PLAIN internal
/// name, while the resolved target key is LOADER-QUALIFIED (`name\0loader:N`). The
/// old `iface == to_key` equality never matched the plain-vs-qualified pair, so a
/// class that genuinely implements the interface reported `false`.
#[test]
fn is_assignable_from_matches_loader_qualified_interface() {
    let iface_key = "com/example/Iface\0loader:7";
    let impl_key = "com/example/Impl\0loader:7";
    let unrelated_key = "com/example/Unrelated\0loader:7";

    let make_ctx = |name: &str, interfaces: Vec<String>| ClassContext {
        class_name: name.to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces,
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Classfile,
    };

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.register(make_ctx(iface_key, Vec::new()));
    // The interface entry is deliberately a PLAIN internal name (no loader
    // qualifier), reproducing the reflective/synthetic build path that never runs
    // the loader-resolution pass.
    registry.register(make_ctx(impl_key, vec!["com/example/Iface".to_string()]));
    registry.register(make_ctx(unrelated_key, Vec::new()));
    let loader = make_simple_loader();

    // Positive: `Impl` implements `Iface` even though the stored interface entry is
    // plain and the resolved target key (`iface_key`) is loader-qualified.
    assert!(
        is_assignable_from(&mut registry, &loader, impl_key, iface_key, Some(impl_key)),
        "plain interface entry must match a loader-qualified target key"
    );

    // Negative guard: a class that does not implement `Iface` must still report
    // false, so the fix is not an unconditional true.
    assert!(
        !is_assignable_from(
            &mut registry,
            &loader,
            unrelated_key,
            iface_key,
            Some(unrelated_key),
        ),
        "an unrelated class must not be assignable to Iface"
    );
}

// ===========================================================================
// Static field / class-init superclass-chain resolution (JVMS §5.4.3.2 / §5.5)
//
// These mirror the invokestatic super-walk fix: `getstatic`/`putstatic` must
// resolve an inherited static by walking the superclass chain and index the
// DECLARING class's per-class `static_fields`, and `ensure_initialized` must
// eagerly initialize the direct superclass before the class itself.
// ===========================================================================

/// FIX (A): `getstatic`/`putstatic` bound to a subclass must resolve a static
/// field declared on a superclass and read/write the superclass's per-class
/// storage. On trunk this raises `InvalidFieldref { index: 0 }`.
#[allow(clippy::too_many_lines)]
#[test]
fn getstatic_resolves_inherited_static_field() {
    use duke_classfile::CpIndex;
    use std::collections::HashMap;
    use std::sync::Arc;

    // Caller CP: a Fieldref symbolically bound to the SUBCLASS `SubNoStatic`
    // for the field `COUNTER:I` that is actually declared on the superclass.
    //   [1]=Fieldref(class=2,nat=3) [2]=Class(name=4) [3]=NameAndType(name=5,desc=6)
    //   [4]=Utf8("SubNoStatic") [5]=Utf8("COUNTER") [6]=Utf8("I")
    let cp = make_cp(vec![
        Some(CpEntry::Fieldref {
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
        Some(CpEntry::Utf8("SubNoStatic".to_string())),
        Some(CpEntry::Utf8("COUNTER".to_string())),
        Some(CpEntry::Utf8("I".to_string())),
    ]);

    // readCounter()I : getstatic SubNoStatic.COUNTER ; ireturn
    let read_instrs: Arc<[(usize, Instruction)]> = vec![
        (0, Instruction::Getstatic(CpIndex(1))),
        (3, Instruction::Ireturn),
    ]
    .into();
    let read_method = MethodEntry {
        name: "readCounter".to_string(),
        descriptor: "()I".to_string(),
        is_public: true,
        is_static: true,
        is_native: false,
        is_abstract: false,
        instructions: Arc::clone(&read_instrs),
        max_stack: 1,
        max_locals: 0,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (3, 1)])),
        line_number_table: Vec::new(),
        source_file: None,
    };

    // writeCounter(I)V : iload0 ; putstatic SubNoStatic.COUNTER ; return
    let write_instrs: Arc<[(usize, Instruction)]> = vec![
        (0, Instruction::Iload0),
        (1, Instruction::Putstatic(CpIndex(1))),
        (4, Instruction::Return),
    ]
    .into();
    let write_method = MethodEntry {
        name: "writeCounter".to_string(),
        descriptor: "(I)V".to_string(),
        is_public: true,
        is_static: true,
        is_native: false,
        is_abstract: false,
        instructions: Arc::clone(&write_instrs),
        max_stack: 1,
        max_locals: 1,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (1, 1), (4, 2)])),
        line_number_table: Vec::new(),
        source_file: None,
    };

    let caller_ctx = ClassContext {
        class_name: "StaticCaller".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: cp,
        methods: vec![read_method, write_method],
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Classfile,
    };

    // Superclass owns the static `COUNTER:I`; its per-class storage holds 42.
    let super_ctx = ClassContext {
        class_name: "SuperWithStatic".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: vec![None],
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "COUNTER".to_string(),
            descriptor: "I".to_string(),
            is_static: true,
        }],
        static_fields: vec![Slot::Int(42)],
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };

    // Subclass declares NO static of its own — storage is empty.
    let sub_ctx = ClassContext {
        class_name: "SubNoStatic".to_string(),
        super_class: Some("SuperWithStatic".to_string()),
        interfaces: Vec::new(),
        constant_pool: vec![None],
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.register(caller_ctx);
    registry.register(super_ctx);
    registry.register(sub_ctx);
    let loader = fixtures_loader();

    // getstatic through the subclass ref reads the superclass's storage.
    let mut sink: Vec<u8> = Vec::new();
    let read = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StaticCaller",
        "readCounter",
        "()I",
        &[],
    )
    .expect("getstatic of an inherited static must resolve via the super chain")
    .expect("readCounter()I must return a value");
    assert_eq!(
        read,
        Slot::Int(42),
        "getstatic SubNoStatic.COUNTER must read the superclass's static storage"
    );

    // putstatic through the subclass ref writes into the superclass's storage.
    execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StaticCaller",
        "writeCounter",
        "(I)V",
        &[Slot::Int(99)],
    )
    .expect("putstatic of an inherited static must resolve via the super chain");

    // The write landed in the DECLARING class, not the subclass.
    assert_eq!(
        registry.get("SuperWithStatic").unwrap().static_fields[0],
        Slot::Int(99),
        "putstatic must write the superclass's per-class static storage"
    );
    assert!(
        registry
            .get("SubNoStatic")
            .unwrap()
            .static_fields
            .is_empty(),
        "the subclass must not gain its own static storage for an inherited field"
    );

    // Reading again observes the updated superclass value.
    let reread = execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "StaticCaller",
        "readCounter",
        "()I",
        &[],
    )
    .expect("re-read must succeed")
    .expect("readCounter()I must return a value");
    assert_eq!(
        reread,
        Slot::Int(99),
        "getstatic must observe the value written through the subclass ref"
    );
}

/// FIX (B): initializing a subclass must eagerly initialize its direct
/// superclass FIRST (JVMS §5.5 step 7). Each `<clinit>` stamps a monotonically
/// increasing sequence number into its own `ORDER` static; the superclass must
/// receive the earlier number. On trunk the superclass's `<clinit>` never runs
/// (nothing else references it), so its `ORDER` stays 0 and the assertion fails.
#[allow(clippy::too_many_lines)]
#[test]
fn subclass_init_initializes_superclass_first() {
    use duke_classfile::CpIndex;
    use std::collections::HashMap;
    use std::sync::Arc;

    // A shared `Log.SEQ:I` counter (starts at 1); each <clinit> reads it into
    // its own ORDER, then increments SEQ. Build a <clinit> body parameterized
    // by the owning class name for the ORDER Fieldref.
    //   CP: [1]=Fieldref Log.SEQ(class=2,nat=3) [2]=Class(4) [3]=NaT(5,6)
    //       [4]=Utf8("Log") [5]=Utf8("SEQ") [6]=Utf8("I")
    //       [7]=Fieldref <owner>.ORDER(class=8,nat=9) [8]=Class(10) [9]=NaT(11,6)
    //       [10]=Utf8(<owner>) [11]=Utf8("ORDER")
    let make_clinit_cp = |owner: &str| {
        make_cp(vec![
            Some(CpEntry::Fieldref {
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
            Some(CpEntry::Utf8("Log".to_string())),
            Some(CpEntry::Utf8("SEQ".to_string())),
            Some(CpEntry::Utf8("I".to_string())),
            Some(CpEntry::Fieldref {
                class_index: CpIndex(8),
                name_and_type_index: CpIndex(9),
            }),
            Some(CpEntry::Class {
                name_index: CpIndex(10),
            }),
            Some(CpEntry::NameAndType {
                name_index: CpIndex(11),
                descriptor_index: CpIndex(6),
            }),
            Some(CpEntry::Utf8(owner.to_string())),
            Some(CpEntry::Utf8("ORDER".to_string())),
        ])
    };
    // <clinit>()V :
    //   getstatic Log.SEQ ; dup ; putstatic <owner>.ORDER ;
    //   iconst1 ; iadd ; putstatic Log.SEQ ; return
    let clinit_method = || {
        let instrs: Arc<[(usize, Instruction)]> = vec![
            (0, Instruction::Getstatic(CpIndex(1))),
            (3, Instruction::Dup),
            (4, Instruction::Putstatic(CpIndex(7))),
            (7, Instruction::Iconst1),
            (8, Instruction::Iadd),
            (9, Instruction::Putstatic(CpIndex(1))),
            (12, Instruction::Return),
        ]
        .into();
        MethodEntry {
            name: "<clinit>".to_string(),
            descriptor: "()V".to_string(),
            is_public: false,
            is_static: true,
            is_native: false,
            is_abstract: false,
            instructions: instrs,
            max_stack: 2,
            max_locals: 0,
            exception_table: Vec::new(),
            pc_to_idx: Arc::new(HashMap::from([
                (0, 0),
                (3, 1),
                (4, 2),
                (7, 3),
                (8, 4),
                (9, 5),
                (12, 6),
            ])),
            line_number_table: Vec::new(),
            source_file: None,
        }
    };

    let super_ctx = ClassContext {
        class_name: "SuperInit".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: make_clinit_cp("SuperInit"),
        methods: vec![clinit_method()],
        fields: vec![FieldEntry {
            name: "ORDER".to_string(),
            descriptor: "I".to_string(),
            is_static: true,
        }],
        static_fields: vec![Slot::Int(0)],
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Classfile,
    };
    let sub_ctx = ClassContext {
        class_name: "SubInit".to_string(),
        super_class: Some("SuperInit".to_string()),
        interfaces: Vec::new(),
        constant_pool: make_clinit_cp("SubInit"),
        methods: vec![clinit_method()],
        fields: vec![FieldEntry {
            name: "ORDER".to_string(),
            descriptor: "I".to_string(),
            is_static: true,
        }],
        static_fields: vec![Slot::Int(0)],
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Classfile,
    };
    let log_ctx = ClassContext {
        class_name: "Log".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: vec![None],
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "SEQ".to_string(),
            descriptor: "I".to_string(),
            is_static: true,
        }],
        static_fields: vec![Slot::Int(1)],
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };

    // Caller triggers SubInit's initialization via getstatic SubInit.ORDER.
    //   [1]=Fieldref(class=2,nat=3) [2]=Class(4) [3]=NaT(5,6)
    //   [4]=Utf8("SubInit") [5]=Utf8("ORDER") [6]=Utf8("I")
    let caller_cp = make_cp(vec![
        Some(CpEntry::Fieldref {
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
        Some(CpEntry::Utf8("SubInit".to_string())),
        Some(CpEntry::Utf8("ORDER".to_string())),
        Some(CpEntry::Utf8("I".to_string())),
    ]);
    let trigger_instrs: Arc<[(usize, Instruction)]> = vec![
        (0, Instruction::Getstatic(CpIndex(1))),
        (3, Instruction::Ireturn),
    ]
    .into();
    let trigger_method = MethodEntry {
        name: "trigger".to_string(),
        descriptor: "()I".to_string(),
        is_public: true,
        is_static: true,
        is_native: false,
        is_abstract: false,
        instructions: trigger_instrs,
        max_stack: 1,
        max_locals: 0,
        exception_table: Vec::new(),
        pc_to_idx: Arc::new(HashMap::from([(0, 0), (3, 1)])),
        line_number_table: Vec::new(),
        source_file: None,
    };
    let caller_ctx = ClassContext {
        class_name: "InitOrderCaller".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        interfaces: Vec::new(),
        constant_pool: caller_cp,
        methods: vec![trigger_method],
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Classfile,
    };

    let mut registry = ClassRegistry::new();
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    registry.register(caller_ctx);
    registry.register(super_ctx);
    registry.register(sub_ctx);
    registry.register(log_ctx);
    let loader = fixtures_loader();

    let mut sink: Vec<u8> = Vec::new();
    execute_class(
        &mut registry,
        &loader,
        &mut heap,
        &mut sink,
        "InitOrderCaller",
        "trigger",
        "()I",
        &[],
    )
    .expect("triggering SubInit initialization must succeed")
    .expect("trigger()I must return a value");

    // Superclass <clinit> ran first (sequence 1); subclass ran second (2).
    assert_eq!(
        registry.get("SuperInit").unwrap().static_fields[0],
        Slot::Int(1),
        "the direct superclass <clinit> must run first (JVMS 5.5 step 7)"
    );
    assert_eq!(
        registry.get("SubInit").unwrap().static_fields[0],
        Slot::Int(2),
        "the subclass <clinit> must run after its superclass's"
    );
}

// ── Stage-c charset writer-graph floor: `JavaLangAccess.encodeASCII` ──
//
// `native_java_lang_access_encode_ascii` is the ASCII fast-path that the real
// `sun.nio.cs.UTF_8$Encoder` reaches via `SharedSecrets.getJavaLangAccess()`. It is
// dormant on the committed tree (the writer graph walls earlier at
// `ByteBuffer.UNSAFE`), so these direct-dispatch tests pin its contract against
// bit-rot, mirroring the `FileOutputStream` floor tests above. It must match the JDK
// `StringCoding.implEncodeAsciiArray`: copy `sa[sp+i]` into `da[dp+i]` as bytes while
// each char is `< 0x80`, stop at the first char `>= 0x80`, and return the count
// encoded.

/// Pure-ASCII input: every char is encoded, the return value is the full length, and
/// each destination byte matches the source char, honouring the `sp`/`dp` offsets.
#[test]
fn native_encode_ascii_pure_ascii_encodes_all() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();

    // sa = { 'X', 'h', 'e', 'l', 'l', 'o' }; encode the 5-char tail from sp = 1.
    let chars = ['X', 'h', 'e', 'l', 'l', 'o'];
    let sa_ref = heap.allocate("[C".to_string(), chars.len());
    for (i, c) in chars.into_iter().enumerate() {
        heap.get_mut(sa_ref).unwrap().fields[i] = Slot::Int(i32::from(c as u16));
    }
    // da has a 2-slot pad so we can check writes land at dp = 2, not 0.
    let da_ref = heap.allocate("[B".to_string(), 7);

    let ret = native_java_lang_access_encode_ascii(
        &[
            Slot::Reference(Some(0)), // receiver (unused)
            Slot::Reference(Some(sa_ref)),
            Slot::Int(1), // sp
            Slot::Reference(Some(da_ref)),
            Slot::Int(2), // dp
            Slot::Int(5), // len
        ],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .unwrap()
    .unwrap();

    assert_eq!(ret, Slot::Int(5), "all five ASCII chars encoded");
    let da = &heap.get(da_ref).unwrap().fields;
    assert_eq!(
        da[2..7],
        [
            java_byte_slot(b'h'),
            java_byte_slot(b'e'),
            java_byte_slot(b'l'),
            java_byte_slot(b'l'),
            java_byte_slot(b'o'),
        ],
        "\"hello\" written at dp = 2"
    );
    assert_eq!(
        da[0..2],
        [Slot::Int(0), Slot::Int(0)],
        "bytes before dp are untouched"
    );
}

/// Mixed input: a non-ASCII char (`0x00E9`, `é`) mid-string stops the fast-path. The
/// return is the ASCII-prefix count, and only the prefix bytes are written — the
/// destination slots at and past the stop index stay zeroed for the multibyte slow
/// path to fill.
#[test]
fn native_encode_ascii_stops_at_non_ascii() {
    let mut heap = duke_gc::Heap::new();
    let mut sink: Vec<u8> = Vec::new();

    // sa = { 'a', 'b', 'é'(0x00E9), 'c' }; the non-ASCII char sits at index 2.
    let sa_ref = heap.allocate("[C".to_string(), 4);
    heap.get_mut(sa_ref).unwrap().fields[0] = Slot::Int(i32::from(b'a'));
    heap.get_mut(sa_ref).unwrap().fields[1] = Slot::Int(i32::from(b'b'));
    heap.get_mut(sa_ref).unwrap().fields[2] = Slot::Int(0x00E9);
    heap.get_mut(sa_ref).unwrap().fields[3] = Slot::Int(i32::from(b'c'));
    let da_ref = heap.allocate("[B".to_string(), 4);

    let ret = native_java_lang_access_encode_ascii(
        &[
            Slot::Reference(Some(0)),
            Slot::Reference(Some(sa_ref)),
            Slot::Int(0), // sp
            Slot::Reference(Some(da_ref)),
            Slot::Int(0), // dp
            Slot::Int(4), // len
        ],
        &mut heap,
        &mut sink,
        &mut NativeControl::default(),
    )
    .unwrap()
    .unwrap();

    assert_eq!(ret, Slot::Int(2), "stops at the non-ASCII char (index 2)");
    let da = &heap.get(da_ref).unwrap().fields;
    assert_eq!(
        da[0..2],
        [java_byte_slot(b'a'), java_byte_slot(b'b')],
        "only the ASCII prefix is written"
    );
    assert_eq!(
        da[2..4],
        [Slot::Int(0), Slot::Int(0)],
        "bytes at and past the non-ASCII char are left for the slow path"
    );
}
