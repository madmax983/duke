import java.sql.Driver;
import java.sql.SQLException;
import java.util.Iterator;
import java.util.ServiceLoader;

public final class ServiceLoaderJdbcSmokeTest {
    public static int miniDriverLoadsAndResponds() throws SQLException {
        Iterator<Driver> iterator = ServiceLoader.load(Driver.class).iterator();
        if (!iterator.hasNext()) {
            return -1;
        }
        Driver driver = iterator.next();
        if (iterator.hasNext()) {
            return -2;
        }
        if (!driver.acceptsURL("jdbc:duke:test")) {
            return -3;
        }
        return driver.getMajorVersion() == 7 && driver.getMinorVersion() == 34 ? 1 : -4;
    }
}
