import com.google.gson.Gson;

public final class GsonSmoke {
    private GsonSmoke() {}

    static final class Pojo {
        int count;
        String name;

        Pojo(int count, String name) {
            this.count = count;
            this.name = name;
        }
    }

    public static void main(String[] args) {
        Gson gson = new Gson();
        Pojo original = new Pojo(7, "duke");
        String json = gson.toJson(original);
        Pojo restored = gson.fromJson(json, Pojo.class);
        System.out.println("gson round-trip: " + json + " -> count=" + restored.count + " name=" + restored.name);
    }
}
