package com.swissarmynoife.sdk

import java.net.URI
import java.net.URLEncoder
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.nio.charset.StandardCharsets
import java.time.Duration
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/** HTTP admin client for SwissArmyNoife (sak334-b). */
class SakClient(
    baseUrl: String? = null,
    private val http: HttpClient = HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(30)).build(),
) {
    val baseUrl: String =
        (baseUrl?.takeIf { it.isNotBlank() } ?: DEFAULT_HTTP).trimEnd('/')

    fun health(): JsonElement = getJson("/health")

    fun listModules(): JsonElement = getJson("/v1/sak/modules")

    fun getModule(id: String): JsonElement =
        getJson("/v1/sak/modules/" + URLEncoder.encode(id, StandardCharsets.UTF_8))

    fun capacity(): JsonElement = getJson("/v1/sak/capacity")

    fun listWork(): JsonElement = getJson("/v1/sak/compute/work")

    fun listNodes(): JsonElement = getJson("/v1/sak/compute/nodes")

    fun computeWork(body: Map<String, Any?>): JsonElement =
        postJson("/v1/sak/compute/work", body)

    fun computeNodes(body: Map<String, Any?>): JsonElement =
        postJson("/v1/sak/compute/nodes", body)

    fun enqueueWork(kind: String, payload: Map<String, Any?>? = null): JsonElement =
        computeWork(
            mapOf(
                "action" to "enqueue",
                "kind" to kind,
                "payload" to (payload ?: emptyMap<String, Any?>()),
            ),
        )

    fun claimWork(nodeId: String): JsonElement =
        computeWork(mapOf("action" to "claim", "node_id" to nodeId))

    fun completeWork(
        workId: String,
        nodeId: String,
        result: Map<String, Any?>? = null,
    ): JsonElement =
        computeWork(
            mapOf(
                "action" to "complete",
                "work_id" to workId,
                "node_id" to nodeId,
                "result" to (result ?: emptyMap<String, Any?>()),
            ),
        )

    fun getWork(workId: String): JsonElement =
        computeWork(mapOf("action" to "get", "work_id" to workId))

    fun requeueWork(workId: String): JsonElement =
        computeWork(mapOf("action" to "requeue", "work_id" to workId))

    fun listWorkFiltered(filters: Map<String, Any?>? = null): JsonElement {
        val body = linkedMapOf<String, Any?>("action" to "list")
        filters?.let { body.putAll(it) }
        return computeWork(body)
    }

    fun listNodesFiltered(filters: Map<String, Any?>? = null): JsonElement {
        val body = linkedMapOf<String, Any?>("action" to "list")
        filters?.let { body.putAll(it) }
        return computeNodes(body)
    }

    fun registerNode(
        label: String,
        caps: List<String>? = null,
        nodeId: String? = null,
        sessionId: String? = null,
    ): JsonElement {
        val body = linkedMapOf<String, Any?>("action" to "register", "label" to label)
        if (caps != null) body["caps"] = caps
        if (!nodeId.isNullOrBlank()) body["node_id"] = nodeId
        if (!sessionId.isNullOrBlank()) body["session_id"] = sessionId
        return computeNodes(body)
    }

    fun heartbeatNode(nodeId: String): JsonElement =
        computeNodes(mapOf("action" to "heartbeat", "node_id" to nodeId))

    private fun getJson(path: String): JsonElement {
        val req =
            HttpRequest.newBuilder(URI.create(baseUrl + path))
                .timeout(Duration.ofSeconds(30))
                .GET()
                .build()
        val res = http.send(req, HttpResponse.BodyHandlers.ofString())
        if (res.statusCode() !in 200..299) {
            throw IllegalStateException("${res.statusCode()}: ${res.body()}")
        }
        return JSON.parseToJsonElement(res.body())
    }

    private fun postJson(path: String, payload: Map<String, Any?>): JsonElement {
        val req =
            HttpRequest.newBuilder(URI.create(baseUrl + path))
                .timeout(Duration.ofSeconds(30))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(toJsonObject(payload).toString()))
                .build()
        val res = http.send(req, HttpResponse.BodyHandlers.ofString())
        if (res.statusCode() !in 200..299) {
            throw IllegalStateException("${res.statusCode()}: ${res.body()}")
        }
        return JSON.parseToJsonElement(res.body())
    }

    companion object {
        private const val DEFAULT_HTTP = "http://127.0.0.1:8787"
        private val JSON = Json { ignoreUnknownKeys = true }

        @Suppress("UNCHECKED_CAST")
        private fun toJsonObject(map: Map<String, Any?>): JsonObject =
            buildJsonObject {
                for ((k, v) in map) {
                    when (v) {
                        null -> put(k, JsonPrimitive(null as String?))
                        is String -> put(k, v)
                        is Number -> put(k, v)
                        is Boolean -> put(k, v)
                        is Map<*, *> -> put(k, toJsonObject(v as Map<String, Any?>))
                        is List<*> -> put(k, Json.parseToJsonElement(Json.encodeToString(kotlinx.serialization.serializer<List<String>>(), v.map { it.toString() })))
                        else -> put(k, v.toString())
                    }
                }
            }
    }
}
