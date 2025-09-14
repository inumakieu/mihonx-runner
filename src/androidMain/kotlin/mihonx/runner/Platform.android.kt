package mihonx.runner

actual fun platform() = "Android"

actual object RustBridge {
    actual fun callUserAgent(ctx: ExtensionContext): String {
        TODO("Not yet implemented")
    }
}