import java.util.HashMap;
import java.util.HashSet;
import java.util.UUID;

public class UuidTest {
    public static int testFromStringRoundTrip() {
        UUID uuid = UUID.fromString("550e8400-e29b-41d4-a716-446655440000");
        return "550e8400-e29b-41d4-a716-446655440000".equals(uuid.toString()) ? 1 : 0;
    }

    public static int testInvalidFromStringThrowsIllegalArgumentException() {
        try {
            UUID.fromString("not-a-uuid");
            return 0;
        } catch (IllegalArgumentException expected) {
            return 1;
        }
    }

    public static int testConstructorPreservesMostSignificantBits() {
        UUID uuid = new UUID(0x550e8400e29b41d4L, 0xa716446655440000L);
        return uuid.getMostSignificantBits() == 0x550e8400e29b41d4L ? 1 : 0;
    }

    public static int testConstructorPreservesLeastSignificantBits() {
        UUID uuid = new UUID(0x550e8400e29b41d4L, 0xa716446655440000L);
        return uuid.getLeastSignificantBits() == 0xa716446655440000L ? 1 : 0;
    }

    public static int testRandomUuidVersionAndVariant() {
        UUID uuid = UUID.randomUUID();
        return uuid != null && uuid.version() == 4 && uuid.variant() == 2 ? 1 : 0;
    }

    public static int testNameUuidMatchesHotSpotReference() {
        UUID uuid = UUID.nameUUIDFromBytes("duke".getBytes());
        return "041cf7cf-23d3-3372-a44b-707505218fb0".equals(uuid.toString()) ? 1 : 0;
    }

    public static int testNameUuidVersionAndVariant() {
        UUID uuid = UUID.nameUUIDFromBytes("duke".getBytes());
        return uuid.version() == 3 && uuid.variant() == 2 ? 1 : 0;
    }

    public static int testEqualsUses128BitValue() {
        UUID left = new UUID(0x550e8400e29b41d4L, 0xa716446655440000L);
        UUID right = UUID.fromString("550e8400-e29b-41d4-a716-446655440000");
        UUID different = new UUID(0x550e8400e29b41d4L, 0xa716446655440001L);
        return left.equals(right) && !left.equals(different) && !left.equals("uuid") ? 1 : 0;
    }

    public static int testHashCodeUses128BitValue() {
        UUID left = new UUID(0x550e8400e29b41d4L, 0xa716446655440000L);
        UUID right = UUID.fromString("550e8400-e29b-41d4-a716-446655440000");
        return left.hashCode() == right.hashCode() ? 1 : 0;
    }

    public static int testHashMapKeyLookupUsesUuidEquality() {
        HashMap<UUID, String> map = new HashMap<>();
        map.put(new UUID(0x550e8400e29b41d4L, 0xa716446655440000L), "duke");
        return "duke".equals(map.get(UUID.fromString("550e8400-e29b-41d4-a716-446655440000"))) ? 1 : 0;
    }

    public static int testHashSetMembershipUsesUuidEquality() {
        HashSet<UUID> set = new HashSet<>();
        set.add(new UUID(0x550e8400e29b41d4L, 0xa716446655440000L));
        set.add(UUID.fromString("550e8400-e29b-41d4-a716-446655440000"));
        return set.size() == 1 && set.contains(new UUID(0x550e8400e29b41d4L, 0xa716446655440000L)) ? 1 : 0;
    }

    public static int testCompareToOrdersByMostThenLeastSignificantBits() {
        UUID smallerMsb = new UUID(1L, 99L);
        UUID largerMsb = new UUID(2L, 0L);
        UUID smallerLsb = new UUID(2L, 3L);
        UUID largerLsb = new UUID(2L, 4L);
        return smallerMsb.compareTo(largerMsb) < 0
                && largerLsb.compareTo(smallerLsb) > 0
                && largerLsb.compareTo(new UUID(2L, 4L)) == 0 ? 1 : 0;
    }

    public static int runAll() {
        int ok = 1;
        ok &= testFromStringRoundTrip();
        ok &= testInvalidFromStringThrowsIllegalArgumentException();
        ok &= testConstructorPreservesMostSignificantBits();
        ok &= testConstructorPreservesLeastSignificantBits();
        ok &= testRandomUuidVersionAndVariant();
        ok &= testNameUuidMatchesHotSpotReference();
        ok &= testNameUuidVersionAndVariant();
        ok &= testEqualsUses128BitValue();
        ok &= testHashCodeUses128BitValue();
        ok &= testHashMapKeyLookupUsesUuidEquality();
        ok &= testHashSetMembershipUsesUuidEquality();
        ok &= testCompareToOrdersByMostThenLeastSignificantBits();
        return ok;
    }
}
