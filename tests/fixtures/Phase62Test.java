import java.time.LocalDate;
import java.time.Duration;
import java.time.Period;
import java.time.Instant;

public class Phase62Test {

    // LocalDate.of / getYear / getMonthValue / getDayOfMonth
    public static int testLocalDateComponents() {
        LocalDate d = LocalDate.of(2024, 3, 15);
        return d.getYear() * 10000 + d.getMonthValue() * 100 + d.getDayOfMonth();
    }

    // LocalDate.plusDays
    public static int testLocalDatePlusDays() {
        LocalDate d = LocalDate.of(2024, 1, 1);
        LocalDate d2 = d.plusDays(31L);
        return d2.getYear() * 10000 + d2.getMonthValue() * 100 + d2.getDayOfMonth();
    }

    // LocalDate.minusDays
    public static int testLocalDateMinusDays() {
        LocalDate d = LocalDate.of(2024, 3, 1);
        LocalDate d2 = d.minusDays(1L);
        // 2024 is leap year: Feb has 29 days
        return d2.getYear() * 10000 + d2.getMonthValue() * 100 + d2.getDayOfMonth();
    }

    // LocalDate.plusMonths
    public static int testLocalDatePlusMonths() {
        LocalDate d = LocalDate.of(2023, 11, 30);
        LocalDate d2 = d.plusMonths(3L);
        return d2.getYear() * 10000 + d2.getMonthValue() * 100 + d2.getDayOfMonth();
    }

    // LocalDate.plusYears
    public static int testLocalDatePlusYears() {
        LocalDate d = LocalDate.of(2020, 6, 15);
        LocalDate d2 = d.plusYears(4L);
        return d2.getYear() * 10000 + d2.getMonthValue() * 100 + d2.getDayOfMonth();
    }

    // LocalDate.isBefore / isAfter / isEqual
    public static int testLocalDateComparisons() {
        LocalDate a = LocalDate.of(2020, 1, 1);
        LocalDate b = LocalDate.of(2021, 1, 1);
        int r = 0;
        if (a.isBefore(b)) r += 1;
        if (b.isAfter(a))  r += 2;
        if (a.isEqual(a))  r += 4;
        return r; // expect 7
    }

    // LocalDate.toEpochDay — 1970-01-01 = epoch 0
    public static int testLocalDateEpochDay() {
        LocalDate epoch = LocalDate.of(1970, 1, 1);
        return (int) epoch.toEpochDay(); // expect 0
    }

    // Duration.ofSeconds / getSeconds / toMinutes / toHours
    public static int testDurationSeconds() {
        Duration d = Duration.ofSeconds(3661L);
        long secs = d.getSeconds();
        long mins = d.toMinutes();
        long hrs  = d.toHours();
        return (int)(secs + mins + hrs); // 3661 + 61 + 1 = 3723
    }

    // Duration.ofMinutes / toDays
    public static int testDurationMinutes() {
        Duration d = Duration.ofMinutes(2880L); // 2 days
        return (int) d.toDays(); // expect 2
    }

    // Duration.plus / minus
    public static int testDurationArithmetic() {
        Duration a = Duration.ofHours(2L);
        Duration b = Duration.ofHours(3L);
        Duration sum = a.plus(b);
        Duration diff = b.minus(a);
        return (int)(sum.toHours() + diff.toHours()); // 5 + 1 = 6
    }

    // Duration.isNegative / isZero
    public static int testDurationFlags() {
        Duration pos  = Duration.ofSeconds(10L);
        Duration zero = Duration.ofSeconds(0L);
        int r = 0;
        if (!pos.isNegative()) r += 1;
        if ( zero.isZero())   r += 2;
        return r; // expect 3
    }

    // Period.of / getYears / getMonths / getDays
    public static int testPeriodComponents() {
        Period p = Period.of(1, 6, 15);
        return p.getYears() * 10000 + p.getMonths() * 100 + p.getDays();
    }

    // Period.ofDays / ofMonths / ofYears
    public static int testPeriodFactories() {
        Period d = Period.ofDays(7);
        Period m = Period.ofMonths(3);
        Period y = Period.ofYears(2);
        return d.getDays() + m.getMonths() * 100 + y.getYears() * 10000;
    }

    // Period.isZero / isNegative
    public static int testPeriodFlags() {
        Period zero = Period.of(0, 0, 0);
        Period pos  = Period.of(1, 0, 0);
        int r = 0;
        if (zero.isZero())      r += 1;
        if (!pos.isZero())      r += 2;
        if (!zero.isNegative()) r += 4;
        return r; // expect 7
    }

    // Instant.ofEpochSecond / getEpochSecond
    public static int testInstantEpochSecond() {
        Instant i = Instant.ofEpochSecond(1_000_000L);
        return (int) i.getEpochSecond(); // expect 1000000
    }

    // Instant.ofEpochMilli / toEpochMilli
    public static int testInstantEpochMilli() {
        Instant i = Instant.ofEpochMilli(5_000L);
        return (int) i.toEpochMilli(); // expect 5000
    }

    // Instant.isBefore / isAfter
    public static int testInstantComparisons() {
        Instant a = Instant.ofEpochSecond(100L);
        Instant b = Instant.ofEpochSecond(200L);
        int r = 0;
        if (a.isBefore(b)) r += 1;
        if (b.isAfter(a))  r += 2;
        return r; // expect 3
    }
}
