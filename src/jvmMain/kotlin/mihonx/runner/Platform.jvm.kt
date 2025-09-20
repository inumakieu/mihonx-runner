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

    actual fun getDexVersion(): String {
        return rustGetDexVersion()
    }

    actual fun installExtension(bytes: ByteArray) {
        rustInstallExtension(bytes)
    }

    actual fun getName(ctx: ExtensionContext): String {
        return rustExtensionGetName(ctx)
    }

    actual fun callMethod(method_name: String): String {
        rustExtensionCallMethod(method_name)

        return "Fake data"
    }

    actual fun isUserAgentEqual(): Boolean {
        return rustExtensionIsUserAgentEqual()
    }

    @JvmStatic
    fun logFromRust(message: String) {
        println("Rust logged: $message")
    }

    external fun nativeInit()
    external fun rustUseExtensionContext(ctx: ExtensionContext): String
    external fun rustInstallExtension(bytes: ByteArray)
    external fun rustExtensionGetName(ctx: ExtensionContext): String
    external fun rustExtensionCallMethod(method_name: String)
    external fun rustExtensionIsUserAgentEqual(): Boolean

    external fun rustGetDexVersion(): String
}