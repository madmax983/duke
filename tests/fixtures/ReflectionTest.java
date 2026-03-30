import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Constructor;

public final class ReflectionTest {
    public static int forNameAndGetName() throws Exception {
        Class<?> cls = Class.forName("ReflectionTarget");
        return cls.getName().equals("ReflectionTarget") ? 1 : 0;
    }

    public static int stringClassLiteralUsesBinaryName() {
        return String.class.getName().equals("java.lang.String") ? 1 : 0;
    }

    public static int declaredMethodsIncludePublicAndPrivate() {
        int sawAdd = 0;
        int sawTimes = 0;
        int sawHidden = 0;
        int sawExplode = 0;
        int sawAddLong = 0;

        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            String name = method.getName();
            if (name.equals("add")) sawAdd = 1;
            if (name.equals("times")) sawTimes = 1;
            if (name.equals("hidden")) sawHidden = 1;
            if (name.equals("explode")) sawExplode = 1;
            if (name.equals("addLong")) sawAddLong = 1;
        }

        return sawAdd + sawTimes + sawHidden + sawExplode + sawAddLong;
    }

    public static int declaredFieldsIncludePublicAndPrivate() {
        int sawBase = 0;
        int sawSecret = 0;

        for (Field field : ReflectionTarget.class.getDeclaredFields()) {
            String name = field.getName();
            if (name.equals("base")) sawBase = 1;
            if (name.equals("secret")) sawSecret = 1;
        }

        return sawBase + sawSecret;
    }

    public static int invokeStaticAdd() throws Exception {
        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            if (method.getName().equals("add")) {
                Object result = method.invoke(null, Integer.valueOf(2), Integer.valueOf(5));
                return ((Integer) result).intValue();
            }
        }
        return -1;
    }

    public static int invokeInstanceTimes() throws Exception {
        ReflectionTarget target = new ReflectionTarget(7, 11);
        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            if (method.getName().equals("times")) {
                Object result = method.invoke(target, Integer.valueOf(3));
                return ((Integer) result).intValue();
            }
        }
        return -1;
    }

    public static int invokeStaticAddLong() throws Exception {
        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            if (method.getName().equals("addLong")) {
                Object result = method.invoke(null, Long.valueOf(2L), Long.valueOf(5L));
                return ((Long) result).longValue() == 7L ? 1 : 0;
            }
        }
        return -1;
    }

    public static int getPublicFieldValue() throws Exception {
        ReflectionTarget target = new ReflectionTarget(7, 11);
        for (Field field : ReflectionTarget.class.getDeclaredFields()) {
            if (field.getName().equals("base")) {
                Object result = field.get(target);
                return ((Integer) result).intValue();
            }
        }
        return -1;
    }

    public static int setPublicFieldValue() throws Exception {
        ReflectionTarget target = new ReflectionTarget(7, 11);
        for (Field field : ReflectionTarget.class.getDeclaredFields()) {
            if (field.getName().equals("base")) {
                field.set(target, Integer.valueOf(19));
                return target.base == 19 ? 1 : 0;
            }
        }
        return -1;
    }

    public static int privateFieldGetRaisesIllegalAccess() throws Exception {
        ReflectionTarget target = new ReflectionTarget(7, 11);
        for (Field field : ReflectionTarget.class.getDeclaredFields()) {
            if (field.getName().equals("secret")) {
                try {
                    field.get(target);
                    return 0;
                } catch (IllegalAccessException e) {
                    return 1;
                }
            }
        }
        return -1;
    }

    public static int privateFieldGetWithAccessibleSucceeds() throws Exception {
        ReflectionTarget target = new ReflectionTarget(7, 11);
        Field field = ReflectionTarget.class.getDeclaredField("secret");
        field.setAccessible(true);
        try {
            return ((Integer) field.get(target)).intValue();
        } catch (IllegalAccessException e) {
            return 0;
        }
    }

    public static int privateFieldSetWithAccessibleWritesBack() throws Exception {
        ReflectionTarget target = new ReflectionTarget(7, 11);
        Field field = ReflectionTarget.class.getDeclaredField("secret");
        field.setAccessible(true);
        try {
            field.set(target, Integer.valueOf(23));
            return ((Integer) field.get(target)).intValue() == 23 ? 1 : 0;
        } catch (IllegalAccessException e) {
            return 0;
        }
    }

    public static int publicFieldWrongTargetRaisesIllegalArgument() throws Exception {
        for (Field field : ReflectionTarget.class.getDeclaredFields()) {
            if (field.getName().equals("base")) {
                try {
                    field.get("not a ReflectionTarget");
                    return 0;
                } catch (IllegalArgumentException e) {
                    return 1;
                }
            }
        }
        return -1;
    }

    public static int declaredFieldByNameFindsPublicAndPrivate() throws Exception {
        Field base = ReflectionTarget.class.getDeclaredField("base");
        Field secret = ReflectionTarget.class.getDeclaredField("secret");
        int sawBase = base.getName().equals("base") ? 1 : 0;
        int sawSecret = secret.getName().equals("secret") ? 1 : 0;
        return sawBase + sawSecret;
    }

    public static int missingDeclaredFieldRaisesNoSuchField() {
        try {
            ReflectionTarget.class.getDeclaredField("ghost");
            return 0;
        } catch (NoSuchFieldException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int inheritedPublicFieldLookupFindsSuperclassMember() throws Exception {
        ReflectionDerivedTarget target = new ReflectionDerivedTarget(5, 13, 17);
        Field field = ReflectionDerivedTarget.class.getField("base");
        return ((Integer) field.get(target)).intValue() == 13 ? 1 : 0;
    }

    public static int inheritedPublicMethodLookupFindsSuperclassMember() throws Exception {
        ReflectionDerivedTarget target = new ReflectionDerivedTarget(5, 13, 17);
        Method method = ReflectionDerivedTarget.class.getMethod("baseValue");
        return ((Integer) method.invoke(target)).intValue() == 13 ? 1 : 0;
    }

    public static int inheritedPrivateFieldIsNotPublicLookupVisible() {
        try {
            ReflectionDerivedTarget.class.getField("hiddenBase");
            return 0;
        } catch (NoSuchFieldException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int inheritedPrivateMethodIsNotPublicLookupVisible() {
        try {
            ReflectionDerivedTarget.class.getMethod("hiddenBaseValue");
            return 0;
        } catch (NoSuchMethodException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int publicFieldsIncludeDeclaredAndInheritedPublicOnly() {
        int sawOwn = 0;
        int sawBase = 0;
        int sawHidden = 0;
        for (Field field : ReflectionDerivedTarget.class.getFields()) {
            String name = field.getName();
            if (name.equals("own")) sawOwn = 1;
            if (name.equals("base")) sawBase = 1;
            if (name.equals("hiddenBase")) sawHidden = 1;
        }
        return sawOwn == 1 && sawBase == 1 && sawHidden == 0 ? 1 : 0;
    }

    public static int publicMethodsIncludeDeclaredAndInheritedPublicOnly() {
        int sawOwnValue = 0;
        int sawBaseValue = 0;
        int sawTimes = 0;
        int sawHiddenBaseValue = 0;
        for (Method method : ReflectionDerivedTarget.class.getMethods()) {
            String name = method.getName();
            if (name.equals("ownValue")) sawOwnValue = 1;
            if (name.equals("baseValue")) sawBaseValue = 1;
            if (name.equals("times")) sawTimes = 1;
            if (name.equals("hiddenBaseValue")) sawHiddenBaseValue = 1;
        }
        return sawOwnValue == 1 && sawBaseValue == 1 && sawTimes == 1 && sawHiddenBaseValue == 0
                ? 1
                : 0;
    }

    public static int fieldDeclaringClassMatchesDeclaredAndInheritedOwners() throws Exception {
        Field declared = ReflectionTarget.class.getDeclaredField("base");
        Field inherited = ReflectionDerivedTarget.class.getField("base");
        return declared.getDeclaringClass().getName().equals("ReflectionTarget")
                        && inherited.getDeclaringClass().getName().equals("ReflectionBaseTarget")
                ? 1
                : 0;
    }

    public static int methodDeclaringClassMatchesDeclaredAndInheritedOwners() throws Exception {
        Method declared = ReflectionTarget.class.getDeclaredMethod("explode");
        Method inherited = ReflectionDerivedTarget.class.getMethod("baseValue");
        return declared.getDeclaringClass().getName().equals("ReflectionTarget")
                        && inherited.getDeclaringClass().getName().equals("ReflectionBaseTarget")
                ? 1
                : 0;
    }

    public static int publicConstructorNewInstanceCreatesObject() throws Exception {
        Constructor<ReflectionCtorTarget> ctor = ReflectionCtorTarget.class.getConstructor(String.class);
        ReflectionCtorTarget target = ctor.newInstance("duke");
        return target.label.equals("duke") && target.secretPlusOne() == 0 ? 1 : 0;
    }

    public static int publicConstructorLookupSkipsPrivateConstructor() {
        try {
            ReflectionCtorTarget.class.getConstructor();
            return 0;
        } catch (NoSuchMethodException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int privateDeclaredConstructorWithAccessibleCreatesObject() throws Exception {
        Constructor<ReflectionCtorTarget> ctor = ReflectionCtorTarget.class.getDeclaredConstructor();
        ctor.setAccessible(true);
        ReflectionCtorTarget target = ctor.newInstance();
        return target.label.equals("hidden") && target.secretPlusOne() == 42 ? 1 : 0;
    }

    public static int declaredConstructorsIncludePublicAndPrivate() {
        int sawPublic = 0;
        int sawPrivate = 0;
        for (Constructor<?> ctor : ReflectionCtorTarget.class.getDeclaredConstructors()) {
            int parameterCount = ctor.getParameterCount();
            if (parameterCount == 1) sawPublic = 1;
            if (parameterCount == 0) sawPrivate = 1;
        }
        return sawPublic + sawPrivate;
    }

    public static int publicConstructorsExcludePrivateOnes() {
        int sawPublic = 0;
        int sawPrivate = 0;
        for (Constructor<?> ctor : ReflectionCtorTarget.class.getConstructors()) {
            int parameterCount = ctor.getParameterCount();
            if (parameterCount == 1) sawPublic = 1;
            if (parameterCount == 0) sawPrivate = 1;
        }
        return sawPublic == 1 && sawPrivate == 0 ? 1 : 0;
    }

    public static int constructorGetNameReturnsDeclaringClassBinaryName() throws Exception {
        Constructor<ReflectionCtorTarget> ctor = ReflectionCtorTarget.class.getDeclaredConstructor();
        return ctor.getName().equals("ReflectionCtorTarget") ? 1 : 0;
    }

    public static int classNewInstanceCreatesObjectViaPublicZeroArgConstructor() throws Exception {
        ReflectionNoArgTarget target = ReflectionNoArgTarget.class.newInstance();
        return target.value.equals("fresh") ? 1 : 0;
    }

    public static int classNewInstancePrivateZeroArgRaisesIllegalAccess() {
        try {
            ReflectionCtorTarget.class.newInstance();
            return 0;
        } catch (IllegalAccessException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int classNewInstanceMissingZeroArgRaisesInstantiation() {
        try {
            ReflectionArgOnlyCtorTarget.class.newInstance();
            return 0;
        } catch (InstantiationException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int fieldGetTypeReturnsPrimitiveReferenceAndArrayMirrors() throws Exception {
        Field primitive = ReflectionTarget.class.getDeclaredField("base");
        Field reference = ReflectionCtorTarget.class.getDeclaredField("label");
        Field array = ReflectionTypeTarget.class.getDeclaredField("names");
        return primitive.getType().getName().equals("int")
                        && reference.getType().getName().equals("java.lang.String")
                        && array.getType().getName().equals("[Ljava.lang.String;")
                ? 1
                : 0;
    }

    public static int methodGetReturnTypeReturnsPrimitiveReferenceAndArrayMirrors()
            throws Exception {
        Method primitive = null;
        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            if (method.getName().equals("times")) {
                primitive = method;
                break;
            }
        }
        if (primitive == null) return -1;
        Method reference = ReflectionTypeTarget.class.getDeclaredMethod("label");
        Method array = ReflectionTypeTarget.class.getDeclaredMethod("names");
        return primitive.getReturnType().getName().equals("int")
                        && reference.getReturnType().getName().equals("java.lang.String")
                        && array.getReturnType().getName().equals("[Ljava.lang.String;")
                ? 1
                : 0;
    }

    public static int methodGetParameterTypesPreserveOrderAndKinds() throws Exception {
        Method target = null;
        for (Method method : ReflectionTypeTarget.class.getDeclaredMethods()) {
            if (method.getName().equals("configure")) {
                target = method;
                break;
            }
        }
        if (target == null) return -1;
        Class<?>[] types = target.getParameterTypes();
        return types.length == 3
                        && types[0].getName().equals("int")
                        && types[1].getName().equals("java.lang.String")
                        && types[2].getName().equals("[Ljava.lang.String;")
                ? 1
                : 0;
    }

    public static int constructorGetParameterTypesPreserveOrderAndKinds() throws Exception {
        Constructor<?> target = null;
        for (Constructor<?> ctor : ReflectionTypeTarget.class.getDeclaredConstructors()) {
            if (ctor.getParameterCount() == 3) {
                target = ctor;
                break;
            }
        }
        if (target == null) return -1;
        Class<?>[] types = target.getParameterTypes();
        return types.length == 3
                        && types[0].getName().equals("int")
                        && types[1].getName().equals("java.lang.String")
                        && types[2].getName().equals("[Ljava.lang.String;")
                ? 1
                : 0;
    }

    public static int primitiveWrapperTypeFieldsReturnExpectedNames() {
        return Integer.TYPE.getName().equals("int")
                        && Long.TYPE.getName().equals("long")
                        && Double.TYPE.getName().equals("double")
                        && Float.TYPE.getName().equals("float")
                        && Boolean.TYPE.getName().equals("boolean")
                        && Character.TYPE.getName().equals("char")
                        && Byte.TYPE.getName().equals("byte")
                        && Short.TYPE.getName().equals("short")
                        && Void.TYPE.getName().equals("void")
                ? 1
                : 0;
    }

    public static int primitiveClassLiteralsAgreeWithWrapperTypeFields() {
        return Integer.TYPE == int.class
                        && Long.TYPE == long.class
                        && Double.TYPE == double.class
                        && Float.TYPE == float.class
                        && Boolean.TYPE == boolean.class
                        && Character.TYPE == char.class
                        && Byte.TYPE == byte.class
                        && Short.TYPE == short.class
                        && Void.TYPE == void.class
                ? 1
                : 0;
    }

    public static int booleanWrapperRoundTrip() {
        return Boolean.valueOf(true).booleanValue() && !Boolean.valueOf(false).booleanValue() ? 1 : 0;
    }

    public static int floatWrapperRoundTrip() {
        return Float.valueOf(2.5f).floatValue() == 2.5f ? 1 : 0;
    }

    public static int byteWrapperRoundTrip() {
        return Byte.valueOf((byte) 7).byteValue() == 7 ? 1 : 0;
    }

    public static int shortWrapperRoundTrip() {
        return Short.valueOf((short) 9).shortValue() == 9 ? 1 : 0;
    }

    @SuppressWarnings({"rawtypes", "unchecked"})
    public static int booleanWrapperCompareToOrdersFalseBeforeTrue() {
        Comparable left = Boolean.valueOf(false);
        return left.compareTo(Boolean.valueOf(true)) < 0 ? 1 : 0;
    }

    @SuppressWarnings({"rawtypes", "unchecked"})
    public static int floatWrapperCompareToOrdersValues() {
        Comparable left = Float.valueOf(1.5f);
        return left.compareTo(Float.valueOf(2.5f)) < 0 ? 1 : 0;
    }

    @SuppressWarnings({"rawtypes", "unchecked"})
    public static int byteWrapperCompareToOrdersValues() {
        Comparable left = Byte.valueOf((byte) 1);
        return left.compareTo(Byte.valueOf((byte) 2)) < 0 ? 1 : 0;
    }

    @SuppressWarnings({"rawtypes", "unchecked"})
    public static int shortWrapperCompareToOrdersValues() {
        Comparable left = Short.valueOf((short) 1);
        return left.compareTo(Short.valueOf((short) 2)) < 0 ? 1 : 0;
    }

    public static int integerWrapperTypedCompareToOrdersValues() {
        return Integer.valueOf(1).compareTo(Integer.valueOf(2)) < 0 ? 1 : 0;
    }

    public static int longWrapperTypedCompareToOrdersValues() {
        return Long.valueOf(1L).compareTo(Long.valueOf(2L)) < 0 ? 1 : 0;
    }

    public static int doubleWrapperTypedCompareToOrdersValues() {
        return Double.valueOf(1.5).compareTo(Double.valueOf(2.5)) < 0 ? 1 : 0;
    }

    public static int floatWrapperTypedCompareToOrdersValues() {
        return Float.valueOf(1.5f).compareTo(Float.valueOf(2.5f)) < 0 ? 1 : 0;
    }

    public static int booleanWrapperTypedCompareToOrdersFalseBeforeTrue() {
        return Boolean.valueOf(false).compareTo(Boolean.valueOf(true)) < 0 ? 1 : 0;
    }

    public static int byteWrapperTypedCompareToOrdersValues() {
        return Byte.valueOf((byte) 1).compareTo(Byte.valueOf((byte) 2)) < 0 ? 1 : 0;
    }

    public static int shortWrapperTypedCompareToOrdersValues() {
        return Short.valueOf((short) 1).compareTo(Short.valueOf((short) 2)) < 0 ? 1 : 0;
    }

    public static int characterWrapperTypedCompareToOrdersValues() {
        return Character.valueOf('a').compareTo(Character.valueOf('b')) < 0 ? 1 : 0;
    }

    public static int byteParseRoundTrip() {
        return Byte.valueOf(Byte.parseByte("7")).byteValue() == 7 ? 1 : 0;
    }

    public static int shortParseRoundTrip() {
        return Short.valueOf(Short.parseShort("9")).shortValue() == 9 ? 1 : 0;
    }

    public static int byteParseRejectsOutOfRange() {
        try {
            Byte.parseByte("128");
            return 0;
        } catch (NumberFormatException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int shortParseRejectsOutOfRange() {
        try {
            Short.parseShort("40000");
            return 0;
        } catch (NumberFormatException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int byteValueOfStringRoundTrip() {
        return Byte.valueOf("7").byteValue() == 7 ? 1 : 0;
    }

    public static int shortValueOfStringRoundTrip() {
        return Short.valueOf("9").shortValue() == 9 ? 1 : 0;
    }

    public static int byteValueOfStringRejectsOutOfRange() {
        try {
            Byte.valueOf("128");
            return 0;
        } catch (NumberFormatException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int shortValueOfStringRejectsOutOfRange() {
        try {
            Short.valueOf("40000");
            return 0;
        } catch (NumberFormatException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int byteParseWithRadixRoundTrip() {
        return Byte.parseByte("7f", 16) == 127 ? 1 : 0;
    }

    public static int shortParseWithRadixRoundTrip() {
        return Short.parseShort("7fff", 16) == 32767 ? 1 : 0;
    }

    public static int byteValueOfStringWithRadixRoundTrip() {
        return Byte.valueOf("7f", 16).byteValue() == 127 ? 1 : 0;
    }

    public static int shortValueOfStringWithRadixRoundTrip() {
        return Short.valueOf("7fff", 16).shortValue() == 32767 ? 1 : 0;
    }

    public static int byteParseWithInvalidRadixRejects() {
        try {
            Byte.parseByte("10", 1);
            return 0;
        } catch (NumberFormatException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int shortValueOfStringWithRadixRejectsOutOfRange() {
        try {
            Short.valueOf("8000", 16);
            return 0;
        } catch (NumberFormatException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int integerParseWithRadixRoundTrip() {
        return Integer.parseInt("7fffffff", 16) == Integer.MAX_VALUE ? 1 : 0;
    }

    public static int longParseWithRadixRoundTrip() {
        return Long.parseLong("7fffffffffffffff", 16) == Long.MAX_VALUE ? 1 : 0;
    }

    public static int integerValueOfStringRoundTrip() {
        return Integer.valueOf("123").intValue() == 123 ? 1 : 0;
    }

    public static int longValueOfStringRoundTrip() {
        return Long.valueOf("456").longValue() == 456L ? 1 : 0;
    }

    public static int integerValueOfStringWithRadixRoundTrip() {
        return Integer.valueOf("7fffffff", 16).intValue() == Integer.MAX_VALUE ? 1 : 0;
    }

    public static int longValueOfStringWithRadixRoundTrip() {
        return Long.valueOf("7fffffffffffffff", 16).longValue() == Long.MAX_VALUE ? 1 : 0;
    }

    public static int integerParseWithInvalidRadixRejects() {
        try {
            Integer.parseInt("10", 1);
            return 0;
        } catch (NumberFormatException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int longValueOfStringWithRadixRejectsOutOfRange() {
        try {
            Long.valueOf("8000000000000000", 16);
            return 0;
        } catch (NumberFormatException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int integerDecodeHexPrefixRoundTrip() {
        return Integer.decode("0x7f").intValue() == 127 ? 1 : 0;
    }

    public static int integerDecodeOctalPrefixRoundTrip() {
        return Integer.decode("017").intValue() == 15 ? 1 : 0;
    }

    public static int integerDecodeNegativeMinHexRoundTrip() {
        return Integer.decode("-0x80000000").intValue() == Integer.MIN_VALUE ? 1 : 0;
    }

    public static int integerDecodeRejectsSignAfterPrefix() {
        try {
            Integer.decode("0x-1");
            return 0;
        } catch (NumberFormatException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int longDecodeHashPrefixRoundTrip() {
        return Long.decode("#7fffffffffffffff").longValue() == Long.MAX_VALUE ? 1 : 0;
    }

    public static int longDecodeLeadingPlusHexRoundTrip() {
        return Long.decode("+0x10").longValue() == 16L ? 1 : 0;
    }

    public static int longDecodeNegativeMinHexRoundTrip() {
        return Long.decode("-0x8000000000000000").longValue() == Long.MIN_VALUE ? 1 : 0;
    }

    public static int longDecodeRejectsPositiveOverflow() {
        try {
            Long.decode("0x8000000000000000");
            return 0;
        } catch (NumberFormatException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int integerToHexStringPositive() {
        return "7f".equals(Integer.toHexString(127)) ? 1 : 0;
    }

    public static int integerToOctalStringPositive() {
        return "17".equals(Integer.toOctalString(15)) ? 1 : 0;
    }

    public static int integerToBinaryStringPositive() {
        return "101".equals(Integer.toBinaryString(5)) ? 1 : 0;
    }

    public static int integerToHexStringNegativeUsesUnsignedBits() {
        return "ffffffff".equals(Integer.toHexString(-1)) ? 1 : 0;
    }

    public static int longToHexStringPositive() {
        return "7f".equals(Long.toHexString(127L)) ? 1 : 0;
    }

    public static int longToOctalStringPositive() {
        return "17".equals(Long.toOctalString(15L)) ? 1 : 0;
    }

    public static int longToBinaryStringPositive() {
        return "101".equals(Long.toBinaryString(5L)) ? 1 : 0;
    }

    public static int longToHexStringNegativeUsesUnsignedBits() {
        return "ffffffffffffffff".equals(Long.toHexString(-1L)) ? 1 : 0;
    }

    public static int integerToUnsignedLongExtendsNegativeInt() {
        return Integer.toUnsignedLong(-1) == 4294967295L ? 1 : 0;
    }

    public static int integerCompareUnsignedOrdersNegativeAsLarger() {
        return Integer.compareUnsigned(-1, 1) > 0 ? 1 : 0;
    }

    public static int longCompareUnsignedOrdersNegativeAsLarger() {
        return Long.compareUnsigned(-1L, 1L) > 0 ? 1 : 0;
    }

    public static int longCompareUnsignedTreatsMinAsPositiveHalfRange() {
        return Long.compareUnsigned(Long.MIN_VALUE, 0L) > 0 ? 1 : 0;
    }

    public static int staticFieldGetTriggersInitialization() throws Exception {
        Class<?> cls =
                Class.forName("ReflectionStaticTarget", false, ReflectionTest.class.getClassLoader());
        Field shared = cls.getDeclaredField("shared");
        Field initCount = cls.getDeclaredField("initCount");
        int sharedValue = ((Integer) shared.get(null)).intValue();
        int initValue = ((Integer) initCount.get(null)).intValue();
        return sharedValue == 17 && initValue == 1 ? 1 : 0;
    }

    public static int staticFieldSetWritesBack() throws Exception {
        Class<?> cls =
                Class.forName("ReflectionStaticTarget", false, ReflectionTest.class.getClassLoader());
        Field shared = cls.getDeclaredField("shared");
        shared.set(null, Integer.valueOf(29));
        return ((Integer) shared.get(null)).intValue() == 29 ? 1 : 0;
    }

    public static int missingClassRaisesClassNotFound() {
        try {
            Class.forName("duke.missing.ReflectionGhost");
            return 0;
        } catch (ClassNotFoundException e) {
            return 1;
        } catch (Throwable t) {
            return -1;
        }
    }

    public static int privateMethodRaisesIllegalAccess() throws Exception {
        ReflectionTarget target = new ReflectionTarget(3, 13);
        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            if (method.getName().equals("hidden")) {
                try {
                    method.invoke(target);
                    return 0;
                } catch (IllegalAccessException e) {
                    return 1;
                }
            }
        }
        return -1;
    }

    public static int privateMethodInvokeWithAccessibleSucceeds() throws Exception {
        ReflectionTarget target = new ReflectionTarget(3, 13);
        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            if (method.getName().equals("hidden")) {
                method.setAccessible(true);
                try {
                    Object result = method.invoke(target);
                    return ((Integer) result).intValue();
                } catch (IllegalAccessException e) {
                    return 0;
                }
            }
        }
        return -1;
    }

    public static int targetExceptionIsWrapped() throws Exception {
        ReflectionTarget target = new ReflectionTarget(1, 2);
        for (Method method : ReflectionTarget.class.getDeclaredMethods()) {
            if (method.getName().equals("explode")) {
                try {
                    method.invoke(target);
                    return 0;
                } catch (InvocationTargetException e) {
                    return 1;
                }
            }
        }
        return -1;
    }
}

final class ReflectionTarget {
    public int base;
    private int secret;

    ReflectionTarget(int base, int secret) {
        this.base = base;
        this.secret = secret;
    }

    public static int add(int a, int b) {
        return a + b;
    }

    public static long addLong(long a, long b) {
        return a + b;
    }

    public int times(int factor) {
        return base * factor;
    }

    private int hidden() {
        return secret;
    }

    public void explode() {
        throw new RuntimeException("boom");
    }
}

final class ReflectionStaticTarget {
    public static int initCount = 0;
    public static int shared = initializeShared();

    private static int initializeShared() {
        initCount += 1;
        return 17;
    }
}

class ReflectionBaseTarget {
    public int base;
    private int hiddenBase;

    ReflectionBaseTarget(int base, int hiddenBase) {
        this.base = base;
        this.hiddenBase = hiddenBase;
    }

    public int times(int factor) {
        return base * factor;
    }

    public int baseValue() {
        return base;
    }

    private int hiddenTimes(int factor) {
        return hiddenBase * factor;
    }

    private int hiddenBaseValue() {
        return hiddenBase;
    }
}

final class ReflectionDerivedTarget extends ReflectionBaseTarget {
    public int own;

    ReflectionDerivedTarget(int own, int base, int hiddenBase) {
        super(base, hiddenBase);
        this.own = own;
    }

    public int ownValue() {
        return own;
    }
}

final class ReflectionCtorTarget {
    public String label;
    private int secret;

    public ReflectionCtorTarget(String label) {
        this.label = label;
        this.secret = -1;
    }

    private ReflectionCtorTarget() {
        this.label = "hidden";
        this.secret = 41;
    }

    public int secretPlusOne() {
        return secret + 1;
    }
}

final class ReflectionNoArgTarget {
    public String value;

    public ReflectionNoArgTarget() {
        this.value = "fresh";
    }
}

final class ReflectionArgOnlyCtorTarget {
    public String value;

    public ReflectionArgOnlyCtorTarget(String value) {
        this.value = value;
    }
}

final class ReflectionTypeTarget {
    private int count;
    public String[] names;
    private String label;

    ReflectionTypeTarget(String[] names, String label) {
        this.count = names.length;
        this.names = names;
        this.label = label;
    }

    ReflectionTypeTarget(int count, String label, String[] names) {
        this.count = count;
        this.names = names;
        this.label = label;
    }

    public String[] names() {
        return names;
    }

    public String label() {
        return label;
    }

    public void configure(int count, String label, String[] names) {
        this.count = count;
        this.label = label;
        this.names = names;
    }
}
