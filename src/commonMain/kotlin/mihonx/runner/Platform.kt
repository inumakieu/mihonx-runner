package mihonx.runner

expect fun platform(): String

interface ExtensionContext {
    fun getUserAgent(): String
}

expect object RustBridge {
    fun callUserAgent(ctx: ExtensionContext): String
}