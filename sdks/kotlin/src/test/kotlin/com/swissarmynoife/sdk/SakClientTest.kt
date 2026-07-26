package com.swissarmynoife.sdk

import com.sun.net.httpserver.HttpServer
import java.net.InetSocketAddress
import java.nio.charset.StandardCharsets
import java.util.concurrent.atomic.AtomicReference
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class SakClientTest {
    @Test
    fun stripsTrailingSlash() {
        val c = SakClient("http://127.0.0.1:8787/")
        assertEquals("http://127.0.0.1:8787", c.baseUrl)
    }

    @Test
    fun health() {
        LocalJsonServer.start("/health", """{"ok":true}""").use { srv ->
            val out = SakClient(srv.baseUrl).health()
            assertEquals(true, out["ok"])
        }
    }

    @Test
    fun listHelpers() {
        data class Case(val path: String, val body: String, val call: (SakClient) -> Map<String, Any?>)
        val cases =
            listOf(
                Case("/v1/sak/modules", """{"modules":[]}""", SakClient::listModules),
                Case("/v1/sak/compute/work", """{"work":[]}""", SakClient::listWork),
                Case("/v1/sak/compute/nodes", """{"nodes":[]}""", SakClient::listNodes),
                Case("/v1/sak/capacity", """{"snapshot":{"total_ram_mb":1}}""", SakClient::capacity),
            )
        for (tc in cases) {
            LocalJsonServer.start(tc.path, tc.body).use { srv ->
                val out = tc.call(SakClient(srv.baseUrl))
                assertTrue(out.isNotEmpty())
            }
        }
    }

    @Test
    fun enqueueWork() {
        val posted = AtomicReference<String>()
        LocalJsonServer.startPost(
            "/v1/sak/compute/work",
            posted,
            """{"action":"enqueue","work":{"status":"queued"}}""",
        ).use { srv ->
            val out = SakClient(srv.baseUrl).enqueueWork("echo", mapOf("n" to 1))
            assertEquals("enqueue", out["action"])
            assertTrue(posted.get().contains(""""action":"enqueue""""))
            assertTrue(posted.get().contains(""""kind":"echo""""))
        }
    }

    @Test
    fun requeueAndClaim() {
        val posted = AtomicReference<String>()
        LocalJsonServer.startPost(
            "/v1/sak/compute/work",
            posted,
            """{"action":"ok","work":{"id":"w1"}}""",
        ).use { srv ->
            val c = SakClient(srv.baseUrl)
            c.requeueWork("w1")
            assertTrue(posted.get().contains("requeue"))
            c.claimWork("n1")
            assertTrue(posted.get().contains("claim"))
            c.completeWork("w1", "n1")
            assertTrue(posted.get().contains("complete"))
            c.getWork("w1")
            assertTrue(posted.get().contains(""""action":"get""""))
        }
    }

    private class LocalJsonServer(
        private val server: HttpServer,
        val baseUrl: String,
    ) : AutoCloseable {
        override fun close() {
            server.stop(0)
        }

        companion object {
            fun start(path: String, json: String): LocalJsonServer {
                val server = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0)
                server.createContext(path) { exchange ->
                    val bytes = json.toByteArray(StandardCharsets.UTF_8)
                    exchange.responseHeaders.add("Content-Type", "application/json")
                    exchange.sendResponseHeaders(200, bytes.size.toLong())
                    exchange.responseBody.use { it.write(bytes) }
                }
                server.start()
                return LocalJsonServer(server, "http://127.0.0.1:${server.address.port}")
            }

            fun startPost(
                path: String,
                bodyOut: AtomicReference<String>,
                json: String,
            ): LocalJsonServer {
                val server = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0)
                server.createContext(path) { exchange ->
                    bodyOut.set(String(exchange.requestBody.readBytes(), StandardCharsets.UTF_8))
                    val bytes = json.toByteArray(StandardCharsets.UTF_8)
                    exchange.responseHeaders.add("Content-Type", "application/json")
                    exchange.sendResponseHeaders(200, bytes.size.toLong())
                    exchange.responseBody.use { it.write(bytes) }
                }
                server.start()
                return LocalJsonServer(server, "http://127.0.0.1:${server.address.port}")
            }
        }
    }
}