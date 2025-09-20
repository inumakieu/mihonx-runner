package mihonx.runner

expect fun platform(): String

interface ExtensionContext {
    fun getUserAgent(): String
}

expect object RustBridge {
    fun callUserAgent(ctx: ExtensionContext): String

    fun getDexVersion(): String

    fun installExtension(bytes: ByteArray)
    fun getName(ctx: ExtensionContext): String
    fun callMethod(method_name: String): String
    fun isUserAgentEqual(): Boolean
}