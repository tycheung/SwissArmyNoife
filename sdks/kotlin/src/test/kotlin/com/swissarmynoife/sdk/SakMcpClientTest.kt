package com.swissarmynoife.sdk

import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import com.sun.net.httpserver.HttpExchange
import com.sun.net.httpserver.HttpServer
import java.net.InetSocketAddress
import java.nio.charset.StandardCharsets
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class SakMcpClientTest {
    private val gson = Gson()
    private val mapType = object : TypeToken<Map<String, Any?>>() {}.type

    @Test
    fun pingNegotiatesSession() {
        val server = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0)
        var n = 0
        server.createContext("/") { exchange ->
            val raw = String(exchange.requestBody.readBytes(), StandardCharsets.UTF_8)
            val body: Map<String, Any?> = gson.fromJson(raw, mapType)
            val method = body["method"]?.toString()
            n++
            when (method) {
                "initialize" ->
                    writeJson(exchange, 200, """{"jsonrpc":"2.0","id":1,"result":{}}""", "sess-kt-1")
                "notifications/initialized" -> {
                    exchange.sendResponseHeaders(202, -1)
                    exchange.close()
                }
                "tools/call" -> {
                    assertEquals("sess-kt-1", exchange.requestHeaders.getFirst("mcp-session-id"))
                    writeJson(
                        exchange,
                        200,
                        """{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"pong"}]}}""",
                        null,
                    )
                }
                else -> {
                    exchange.sendResponseHeaders(500, -1)
                    exchange.close()
                }
            }
        }
        server.start()
        try {
            val mcp = SakMcpClient("http://127.0.0.1:${server.address.port}/")
            assertEquals("pong", mcp.ping())
            assertEquals("sess-kt-1", mcp.sessionId)
            assertTrue(n >= 3)
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun toolsListNoAutoInit() {
        val server = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0)
        server.createContext("/") { exchange ->
            val raw = String(exchange.requestBody.readBytes(), StandardCharsets.UTF_8)
            val body: Map<String, Any?> = gson.fromJson(raw, mapType)
            assertEquals("tools/list", body["method"])
            writeJson(exchange, 200, """{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}""", null)
        }
        server.start()
        try {
            val mcp =
                SakMcpClient("http://127.0.0.1:${server.address.port}").apply {
                    autoInitialize = false
                }
            @Suppress("UNCHECKED_CAST")
            val out = mcp.toolsList() as Map<String, Any?>
            assertTrue(out.containsKey("tools"))
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun catalogList() {
        val server = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0)
        server.createContext("/") { exchange ->
            val raw = String(exchange.requestBody.readBytes(), StandardCharsets.UTF_8)
            val body: Map<String, Any?> = gson.fromJson(raw, mapType)
            when (body["method"]?.toString()) {
                "initialize" ->
                    writeJson(exchange, 200, """{"jsonrpc":"2.0","id":1,"result":{}}""", "s2")
                "notifications/initialized" -> {
                    exchange.sendResponseHeaders(202, -1)
                    exchange.close()
                }
                else ->
                    writeJson(exchange, 200, """{"jsonrpc":"2.0","id":2,"result":{"offers":[]}}""", null)
            }
        }
        server.start()
        try {
            val mcp = SakMcpClient("http://127.0.0.1:${server.address.port}")
            @Suppress("UNCHECKED_CAST")
            val out = mcp.catalogList() as Map<String, Any?>
            assertTrue(out.containsKey("offers"))
        } finally {
            server.stop(0)
        }
    }

    private fun writeJson(exchange: HttpExchange, code: Int, json: String, session: String?) {
        if (session != null) {
            exchange.responseHeaders.add("mcp-session-id", session)
        }
        exchange.responseHeaders.add("Content-Type", "application/json")
        val bytes = json.toByteArray(StandardCharsets.UTF_8)
        exchange.sendResponseHeaders(code, bytes.size.toLong())
        exchange.responseBody.use { it.write(bytes) }
    }
}
