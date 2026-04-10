import java.time.LocalDate;
import java.time.LocalDateTime;

public class Phase63Test {

    // LocalDateTime.of(y,m,d,h,min) / getYear/getHour
    public static int testLocalDateTimeComponents() {
        LocalDateTime dt = LocalDateTime.of(2024, 6, 15, 10, 30);
        return dt.getYear() * 10000 + dt.getMonthValue() * 100 + dt.getDayOfMonth();
    }

    // getHour / getMinute / getSecond
    public static int testLocalDateTimeTime() {
        LocalDateTime dt = LocalDateTime.of(2024, 1, 1, 23, 45, 59);
        return dt.getHour() * 10000 + dt.getMinute() * 100 + dt.getSecond();
    }

    // toLocalDate
    public static int testLocalDateTimeToLocalDate() {
        LocalDateTime dt = LocalDateTime.of(2023, 12, 25, 0, 0, 0);
        LocalDate d = dt.toLocalDate();
        return d.getYear() * 10000 + d.getMonthValue() * 100 + d.getDayOfMonth();
    }

    // isBefore / isAfter on same day (time comparison)
    public static int testLocalDateTimeOrdering() {
        LocalDateTime a = LocalDateTime.of(2024, 1, 1, 8, 0, 0);
        LocalDateTime b = LocalDateTime.of(2024, 1, 1, 12, 0, 0);
        LocalDateTime c = LocalDateTime.of(2024, 1, 2, 8, 0, 0);
        int r = 0;
        if (a.isBefore(b)) r += 1; // same day, earlier time
        if (b.isAfter(a))  r += 2;
        if (b.isBefore(c)) r += 4; // different day
        if (c.isAfter(a))  r += 8;
        return r; // expect 15
    }

    // toString — ISO-8601 length check
    public static int testLocalDateTimeToString() {
        LocalDateTime dt = LocalDateTime.of(2024, 3, 15, 9, 5, 7);
        String s = dt.toString(); // "2024-03-15T09:05:07"
        return s.length(); // expect 19
    }

    // plusDays
    public static int testLocalDateTimePlusDays() {
        LocalDateTime dt = LocalDateTime.of(2024, 1, 30, 14, 0, 0);
        LocalDateTime dt2 = dt.plusDays(2L);
        return dt2.getYear() * 10000 + dt2.getMonthValue() * 100 + dt2.getDayOfMonth();
    }

    // withHour
    public static int testLocalDateTimeWithHour() {
        LocalDateTime dt = LocalDateTime.of(2024, 6, 1, 10, 30, 0);
        LocalDateTime dt2 = dt.withHour(18);
        return dt2.getHour(); // expect 18
    }

    // now() — interpreter returns 1970-01-01T00:00:00
    public static int testLocalDateTimeNow() {
        LocalDateTime dt = LocalDateTime.now();
        return dt.getYear(); // expect 1970
    }
}
