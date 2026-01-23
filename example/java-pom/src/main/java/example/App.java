package example;

public class App {
    public static void main(String[] args) {
        System.out.println("Hello from jbundle!");
        System.out.println("Arguments: " + java.util.Arrays.toString(args));
        System.out.println("Java version: " + System.getProperty("java.version"));
        System.out.println("OS: " + System.getProperty("os.name") + " " + System.getProperty("os.arch"));
    }
}
