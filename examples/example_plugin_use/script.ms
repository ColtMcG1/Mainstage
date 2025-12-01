import "cpp" as cpp_plugin;
workspace hello_ws {
    srcs = ["./hello.ms"];
    val = cpp_plugin.compile(srcs);
    say(val);
}