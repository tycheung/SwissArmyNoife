package com.swissarmynoife.sdk

import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.time.Duration

/** Streamable HTTP MCP client (sak334-c). */
class SakMcpClient(
    baseUrl: String? = null,
    private val http: HttpClient =
        HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(30)).build(),
) {
    val baseUrl: String =
        (baseUrl?.takeIf { it.isNotBlank() } ?: DEFAULT_MCP).trimEnd('/')

    var token: String? = null
    var autoInitialize: Boolean = true

    private var rpcId = 0
    private var sessionIdInternal: String? = null
    private var initialized = false

    val sessionId: String?
        get() = sessionIdInternal

    fun initialize(): Any? {
        val result =
            rpc(
                "initialize",
                mapOf(
                    "protocolVersion" to PROTOCOL,
                    "capabilities" to emptyMap<String, Any?>(),
                    "clientInfo" to
                        mapOf(
                            "name" to "swissarmynoife-kotlin",
                            "version" to "0.1.0",
                        ),
                ),
            )
        post(mapOf("jsonrpc" to "2.0", "method" to "notifications/initialized"), notification = true)
        initialized = true
        return result
    }

    fun ping(): String = extractPingText(toolsCall("ping", emptyMap()))

    fun toolsList(): Any? {
        ensureSession()
        return rpc("tools/list", emptyMap())
    }

    fun catalogList(): Any? = toolsCall("catalog_list", emptyMap())

    private fun ensureSession() {
        if (!autoInitialize || initialized) return
        initialize()
    }

    private fun toolsCall(name: String, arguments: Map<String, Any?>): Any? {
        ensureSession()
        return rpc("tools/call", mapOf("name" to name, "arguments" to arguments))
    }

    private fun rpc(method: String, params: Map<String, Any?>): Any? {
        rpcId++
        val payload =
            mapOf(
                "jsonrpc" to "2.0",
                "id" to rpcId,
                "method" to method,
                "params" to params,
            )
        val res = post(payload, notification = false)
        val body: Map<String, Any?> = GSON.fromJson(res.body(), MAP_TYPE)
        captureSession(res, body)
        if (body.containsKey("error")) {
            val err = body["error"]
            val msg =
                if (err is Map<*, *>) err["message"]?.toString() ?: err.toString()
                else err.toString()
            throw IllegalStateException("MCP $method failed: $msg")
        }
        return body["result"] ?: body
    }

    private fun post(
        payload: Map<String, Any?>,
        notification: Boolean,
    ): HttpResponse<String> {
        val builder =
            HttpRequest.newBuilder(URI.create(baseUrl))
                .timeout(Duration.ofSeconds(30))
                .header("Content-Type", "application/json")
                .header("Accept", ACCEPT)
                .POST(HttpRequest.BodyPublishers.ofString(GSON.toJson(payload)))
        token?.takeIf { it.isNotBlank() }?.let { builder.header("Authorization", "Bearer $it") }
        sessionIdInternal?.takeIf { it.isNotBlank() }?.let { builder.header(SESSION_HEADER, it) }
        val res = http.send(builder.build(), HttpResponse.BodyHandlers.ofString())
        if (notification && res.statusCode() in listOf(200, 202)) return res
        if (res.statusCode() !in 200..299) {
            throw IllegalStateException("${res.statusCode()}: ${res.body()}")
        }
        return res
    }

    private fun captureSession(res: HttpResponse<String>, body: Map<String, Any?>) {
        if (sessionIdInternal.isNullOrBlank()) {
            res.headers().firstValue(SESSION_HEADER).ifPresent { sid ->
                if (sid.isNotBlank()) sessionIdInternal = sid.trim()
            }
        }
        if (sessionIdInternal.isNullOrBlank()) {
            sessionIdFromBody(body)?.let { sessionIdInternal = it }
        }
    }

    companion object {
        private const val DEFAULT_MCP = "http://127.0.0.1:8080/mcp"
        private const val PROTOCOL = "2024-11-05"
        private const val SESSION_HEADER = "mcp-session-id"
        private const val ACCEPT = "application/json, text/event-stream"
        private val GSON = Gson()
        private val MAP_TYPE = object : TypeToken<Map<String, Any?>>() {}.type

        @Suppress("UNCHECKED_CAST")
        private fun sessionIdFromBody(body: Map<String, Any?>): String? {
            for (key in listOf("sessionId", "session_id", "mcp-session-id")) {
                val v = body[key]
                if (v is String && v.isNotBlank()) return v.trim()
            }
            val result = body["result"]
            if (result is Map<*, *>) {
                val resultMap = result as Map<String, Any?>
                for (key in listOf("sessionId", "session_id", "mcp-session-id")) {
                    val v = resultMap[key]
                    if (v is String && v.isNotBlank()) return v.trim()
                }
            }
            return null
        }

        @Suppress("UNCHECKED_CAST")
        private fun extractPingText(result: Any?): String {
            if (result is String) return result
            if (result !is Map<*, *>) return result.toString()
            val content = result["content"]
            if (content is List<*>) {
                for (item in content) {
                    if (item is Map<*, *>) {
                        val text = item["text"]
                        if (text is String) return text
                    }
                }
            }
            return result.toString()
        }
    }
}
