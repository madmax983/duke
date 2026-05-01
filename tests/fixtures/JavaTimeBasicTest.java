import java.time.DateTimeException;
import java.time.Duration;
import java.time.Instant;
import java.time.LocalDate;
import java.time.LocalDateTime;
import java.time.format.DateTimeFormatter;
import java.time.format.DateTimeParseException;

public final class JavaTimeBasicTest {
    private JavaTimeBasicTest() {
    }

    public static int instantEpochSecondAndNano() {
        Instant instant = Instant.ofEpochSecond(1_700_000_000L, 500_000_000L);
        return instant.getEpochSecond() == 1_700_000_000L && instant.getNano() == 500_000_000 ? 1 : 0;
    }

    public static int instantArithmeticIsImmutable() {
        Instant base = Instant.ofEpochSecond(100L, 123_000_000L);
        Instant moved = base.plusSeconds(5L).plusNanos(2_000_000L).minusMillis(3L);
        return base.getEpochSecond() == 100L
            && base.getNano() == 123_000_000
            && moved.getEpochSecond() == 105L
            && moved.getNano() == 122_000_000
            && base != moved ? 1 : 0;
    }

    public static int instantParseRoundTrip() {
        Instant parsed = Instant.parse("2026-04-30T12:34:56Z");
        return parsed.toString().equals("2026-04-30T12:34:56Z") ? 1 : 0;
    }

    public static int instantNowWithinSystemMillisWindow() {
        long before = System.currentTimeMillis();
        long actual = Instant.now().toEpochMilli();
        long after = System.currentTimeMillis();
        return before <= actual && actual <= after ? 1 : 0;
    }

    public static int instantCompareEqualsHashCode() {
        Instant a = Instant.ofEpochMilli(5_000L);
        Instant b = Instant.ofEpochSecond(5L);
        Instant c = Instant.ofEpochMilli(7_000L);
        return a.equals(b) && a.hashCode() == b.hashCode() && a.compareTo(c) < 0 && c.compareTo(a) > 0 ? 1 : 0;
    }

    public static int durationFactoriesAndAccessors() {
        Duration millis = Duration.ofMillis(1_500L);
        Duration nanos = Duration.ofNanos(2_500_000L);
        return millis.getSeconds() == 1L
            && millis.toMillis() == 1_500L
            && millis.toNanos() == 1_500_000_000L
            && nanos.toMillis() == 2L
            && nanos.toNanos() == 2_500_000L ? 1 : 0;
    }

    public static int durationBetweenInstants() {
        Duration delta = Duration.between(
            Instant.ofEpochSecond(10L, 250_000_000L),
            Instant.ofEpochSecond(12L, 750_000_000L)
        );
        return delta.toMillis() == 2_500L ? 1 : 0;
    }

    public static int durationArithmeticIsImmutable() {
        Duration base = Duration.ofMillis(1_500L);
        Duration plus = base.plus(Duration.ofMillis(500L));
        Duration minus = plus.minus(Duration.ofSeconds(1L));
        Duration negated = minus.negated();
        return base.toMillis() == 1_500L
            && plus.toMillis() == 2_000L
            && minus.toMillis() == 1_000L
            && negated.toMillis() == -1_000L
            && base != plus
            && plus != minus
            && minus != negated ? 1 : 0;
    }

    public static int durationCompareEqualsHashCode() {
        Duration a = Duration.ofMillis(1_500L);
        Duration b = Duration.ofSeconds(1L).plus(Duration.ofMillis(500L));
        Duration c = Duration.ofSeconds(2L);
        return a.equals(b) && a.hashCode() == b.hashCode() && a.compareTo(c) < 0 && c.compareTo(a) > 0 ? 1 : 0;
    }

    public static int localDateArithmeticIsImmutable() {
        LocalDate base = LocalDate.of(2026, 4, 30);
        LocalDate plus = base.plusDays(1L);
        LocalDate minus = base.minusMonths(1L);
        LocalDate withYear = base.withYear(2027);
        return base.getYear() == 2026
            && base.getMonthValue() == 4
            && base.getDayOfMonth() == 30
            && plus.toString().equals("2026-05-01")
            && minus.toString().equals("2026-03-30")
            && withYear.toString().equals("2027-04-30")
            && base != plus
            && base != minus
            && base != withYear ? 1 : 0;
    }

    public static int localDateParseRoundTrip() {
        LocalDate parsed = LocalDate.parse("2026-04-30");
        return parsed.toString().equals("2026-04-30")
            && parsed.equals(LocalDate.parse(parsed.toString())) ? 1 : 0;
    }

    public static int localDateCompareEqualsHashCode() {
        LocalDate a = LocalDate.of(2026, 4, 30);
        LocalDate b = LocalDate.parse("2026-04-30");
        LocalDate c = LocalDate.of(2026, 5, 1);
        return a.equals(b) && a.hashCode() == b.hashCode() && a.compareTo(c) < 0 && c.compareTo(a) > 0 ? 1 : 0;
    }

    public static int localDateTimeArithmeticIsImmutable() {
        LocalDateTime base = LocalDateTime.of(2026, 4, 30, 23, 0, 0);
        LocalDateTime plusHours = base.plusHours(2L);
        LocalDateTime plusDays = base.plusDays(1L);
        return base.toString().equals("2026-04-30T23:00:00")
            && plusHours.toString().equals("2026-05-01T01:00:00")
            && plusDays.toString().equals("2026-05-01T23:00:00")
            && base != plusHours
            && base != plusDays ? 1 : 0;
    }

    public static int localDateTimeParseRoundTrip() {
        LocalDateTime parsed = LocalDateTime.parse("2026-04-30T12:34:56");
        return parsed.toString().equals("2026-04-30T12:34:56")
            && parsed.equals(LocalDateTime.parse(parsed.toString())) ? 1 : 0;
    }

    public static int localDateTimeCompareEqualsHashCode() {
        LocalDateTime a = LocalDateTime.of(2026, 4, 30, 12, 34, 56);
        LocalDateTime b = LocalDateTime.parse("2026-04-30T12:34:56");
        LocalDateTime c = LocalDateTime.of(2026, 4, 30, 14, 0, 0);
        return a.equals(b) && a.hashCode() == b.hashCode() && a.compareTo(c) < 0 && c.compareTo(a) > 0 ? 1 : 0;
    }

    public static int dateTimeFormatterStaticsResolve() {
        return DateTimeFormatter.ISO_INSTANT != null
            && DateTimeFormatter.ISO_LOCAL_DATE != null
            && DateTimeFormatter.ISO_LOCAL_DATE_TIME != null ? 1 : 0;
    }

    public static int malformedParseRaisesDateTimeParseException() {
        try {
            LocalDate.parse("definitely-not-a-date");
            return 0;
        } catch (DateTimeParseException e) {
            return e instanceof DateTimeException && e instanceof RuntimeException ? 1 : 0;
        }
    }
}
