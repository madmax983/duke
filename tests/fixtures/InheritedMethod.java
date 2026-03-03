public class InheritedMethod {
    static class Animal {
        int sound() { return 1; }
        int move() { return 10; }
    }

    static class Dog extends Animal {
        // Overrides sound, inherits move()
        int sound() { return 2; }
    }

    static class Puppy extends Dog {
        // Inherits both sound() from Dog and move() from Animal
    }

    /** Calls inherited method (move defined in Animal, called on Dog). */
    public static int callInherited() {
        Dog d = new Dog();
        return d.move(); // 10 — inherited from Animal
    }

    /** Calls overridden method. */
    public static int callOverridden() {
        Dog d = new Dog();
        return d.sound(); // 2 — overridden in Dog
    }

    /** Two levels of inheritance. */
    public static int callDeepInherited() {
        Puppy p = new Puppy();
        return p.move(); // 10 — inherited from Animal via Dog
    }

    /** Override at middle level. */
    public static int callDeepOverridden() {
        Puppy p = new Puppy();
        return p.sound(); // 2 — inherited from Dog
    }
}
