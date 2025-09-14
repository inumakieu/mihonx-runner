package mihonx.runner

actual fun platform(): String {
    return "JVM Desktop"
}

actual object RustBridge {
    init {
        System.loadLibrary("mihon_runner")
        nativeInit()
    }

    actual fun callUserAgent(ctx: ExtensionContext): String {
        return rustUseExtensionContext(ctx)
    }

    @JvmStatic
    fun logFromRust(message: String) {
        println("Rust logged: $message")
    }

    external fun nativeInit()
    external fun rustUseExtensionContext(ctx: ExtensionContext): String
}