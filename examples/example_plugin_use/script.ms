import "cpp" as cpp_plugin;

workspace hello_ws {
    compilers = cpp_plugin.list_compilers();
    say("Detected compilers:");
    for compiler in compilers {
        say(" - " + compiler.name + " at " + compiler.path);
    }

    val = cpp_plugin.compile(example_plugin_use.srcs);
    say(val);
}

project example_plugin_use {
    srcs = [ "./hello.ms" ];
    flags = ["-O2"];
}