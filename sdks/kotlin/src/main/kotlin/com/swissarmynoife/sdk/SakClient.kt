package com.swissarmynoife.sdk

import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import java.net.URI
import java.net.URLEncoder
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.nio.charset.StandardCharsets
import java.time.Duration

/** HTTP admin client for SwissArmyNoife (sak334-b). */
class SakClient(
    baseUrl: String? = null,
    private val http: HttpClient =
        HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(30)).build(),
) {
    val baseUrl: String =
        (baseUrl?.takeIf { it.isNotBlank() } ?: DEFAULT_HTTP).trimEnd('/')

    fun health(): Map<String, Any?> = getJson("/health")

    fun listModules(): Map<String, Any?> = getJson("/v1/sak/modules")

    fun getModule(id: String): Map<String, Any?> =
        getJson("/v1/sak/modules/" + URLEncoder.encode(id, StandardCharsets.UTF_8))

    fun capacity(): Map<String, Any?> = getJson("/v1/sak/capacity")

    fun listWork(): Map<String, Any?> = getJson("/v1/sak/compute/work")

    fun listNodes(): Map<String, Any?> = getJson("/v1/sak/compute/nodes")

    fun computeWork(body: Map<String, Any?>): Map<String, Any?> =
        postJson("/v1/sak/compute/work", body)

    fun computeNodes(body: Map<String, Any?>): Map<String, Any?> =
        postJson("/v1/sak/compute/nodes", body)

    fun enqueueWork(kind: String, payload: Map<String, Any?>? = null): Map<String, Any?> =
        computeWork(
            mapOf(
                "action" to "enqueue",
                "kind" to kind,
                "payload" to (payload ?: emptyMap()),
            ),
        )

    fun claimWork(nodeId: String): Map<String, Any?> =
        computeWork(mapOf("action" to "claim", "node_id" to nodeId))

    fun completeWork(
        workId: String,
        nodeId: String,
        result: Map<String, Any?>? = null,
    ): Map<String, Any?> =
        computeWork(
            mapOf(
                "action" to "complete",
                "work_id" to workId,
                "node_id" to nodeId,
                "result" to (result ?: emptyMap()),
            ),
        )

    fun getWork(workId: String): Map<String, Any?> =
        computeWork(mapOf("action" to "get", "work_id" to workId))

    fun requeueWork(workId: String): Map<String, Any?> =
        computeWork(mapOf("action" to "requeue", "work_id" to workId))

    fun listWorkFiltered(filters: Map<String, Any?>? = null): Map<String, Any?> {
        val body = linkedMapOf<String, Any?>("action" to "list")
        filters?.let { body.putAll(it) }
        return computeWork(body)
    }

    fun listNodesFiltered(filters: Map<String, Any?>? = null): Map<String, Any?> {
        val body = linkedMapOf<String, Any?>("action" to "list")
        filters?.let { body.putAll(it) }
        return computeNodes(body)
    }

    fun registerNode(
        label: String,
        caps: List<String>? = null,
        nodeId: String? = null,
        sessionId: String? = null,
    ): Map<String, Any?> {
        val body = linkedMapOf<String, Any?>("action" to "register", "label" to label)
        if (caps != null) body["caps"] = caps
        if (!nodeId.isNullOrBlank()) body["node_id"] = nodeId
        if (!sessionId.isNullOrBlank()) body["session_id"] = sessionId
        return computeNodes(body)
    }

    fun heartbeatNode(nodeId: String): Map<String, Any?> =
        computeNodes(mapOf("action" to "heartbeat", "node_id" to nodeId))

    private fun getJson(path: String): Map<String, Any?> {
        val req =
            HttpRequest.newBuilder(URI.create(baseUrl + path))
                .timeout(Duration.ofSeconds(30))
                .GET()
                .build()
        val res = http.send(req, HttpResponse.BodyHandlers.ofString())
        if (res.statusCode() !in 200..299) {
            throw IllegalStateException("${res.statusCode()}: ${res.body()}")
        }
        return GSON.fromJson(res.body(), MAP_TYPE)
    }

    private fun postJson(path: String, payload: Map<String, Any?>): Map<String, Any?> {
        val req =
            HttpRequest.newBuilder(URI.create(baseUrl + path))
                .timeout(Duration.ofSeconds(30))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(GSON.toJson(payload)))
                .build()
        val res = http.send(req, HttpResponse.BodyHandlers.ofString())
        if (res.statusCode() !in 200..299) {
            throw IllegalStateException("${res.statusCode()}: ${res.body()}")
        }
        return GSON.fromJson(res.body(), MAP_TYPE)
    }

    companion object {
        private const val DEFAULT_HTTP = "http://127.0.0.1:8787"
        private val GSON = Gson()
        private val MAP_TYPE = object : TypeToken<Map<String, Any?>>() {}.type
    }
}
