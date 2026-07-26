package com.swissarmynoife.sdk

fun main() {
    val base = System.getenv("SAK_HTTP") ?: "http://127.0.0.1:8787"
    val sak = SakClient(base)
    println("health=${sak.health()}")
    println("modules=${sak.listModules()}")
}
