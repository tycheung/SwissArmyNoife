<?php

declare(strict_types=1);

namespace SwissArmyNoife\Sdk;

use GuzzleHttp\Client;
use GuzzleHttp\Exception\GuzzleException;
use Psr\Http\Message\ResponseInterface;

/** Streamable HTTP MCP client (sak335-c). */
final class SakMcpClient
{
    private const DEFAULT_MCP = 'http://127.0.0.1:8080/mcp';
    private const PROTOCOL = '2024-11-05';
    private const SESSION_HEADER = 'mcp-session-id';
    private const ACCEPT = 'application/json, text/event-stream';

    private string $baseUrl;
    private Client $http;
    private ?string $token = null;
    private bool $autoInitialize = true;
    private int $rpcId = 0;
    private ?string $sessionId = null;
    private bool $initialized = false;

    public function __construct(?string $baseUrl = null, ?Client $http = null)
    {
        $u = ($baseUrl === null || trim($baseUrl) === '') ? self::DEFAULT_MCP : $baseUrl;
        $this->baseUrl = rtrim($u, '/');
        $this->http = $http ?? new Client(['timeout' => 30.0, 'http_errors' => false]);
    }

    public function setToken(?string $token): void
    {
        $this->token = $token;
    }

    public function setAutoInitialize(bool $autoInitialize): void
    {
        $this->autoInitialize = $autoInitialize;
    }

    public function getSessionId(): ?string
    {
        return $this->sessionId;
    }

    /** @return mixed */
    public function initialize(): mixed
    {
        $result = $this->rpc('initialize', [
            'protocolVersion' => self::PROTOCOL,
            'capabilities' => new \stdClass(),
            'clientInfo' => [
                'name' => 'swissarmynoife-php',
                'version' => SdkInfo::VERSION,
            ],
        ]);
        $this->post(['jsonrpc' => '2.0', 'method' => 'notifications/initialized'], true);
        $this->initialized = true;

        return $result;
    }

    public function ping(): string
    {
        return $this->extractPingText($this->toolsCall('ping'));
    }

    /** @return mixed */
    public function toolsList(): mixed
    {
        $this->ensureSession();

        return $this->rpc('tools/list');
    }

    /** @return mixed */
    public function catalogList(): mixed
    {
        return $this->toolsCall('catalog_list');
    }

    private function ensureSession(): void
    {
        if (!$this->autoInitialize || $this->initialized) {
            return;
        }
        $this->initialize();
    }

    /**
     * @param array<string, mixed> $arguments
     * @return mixed
     */
    private function toolsCall(string $name, array $arguments = []): mixed
    {
        $this->ensureSession();

        return $this->rpc('tools/call', ['name' => $name, 'arguments' => $arguments]);
    }

    /**
     * @param array<string, mixed> $params
     * @return mixed
     */
    private function rpc(string $method, array $params = []): mixed
    {
        $this->rpcId++;
        $payload = [
            'jsonrpc' => '2.0',
            'id' => $this->rpcId,
            'method' => $method,
            'params' => $params,
        ];
        $res = $this->post($payload, false);
        /** @var array<string, mixed> $body */
        $body = json_decode((string) $res->getBody(), true, 512, JSON_THROW_ON_ERROR);
        $this->captureSession($res, $body);
        if (isset($body['error'])) {
            $err = $body['error'];
            $msg = is_array($err) ? ($err['message'] ?? json_encode($err)) : (string) $err;
            throw new \RuntimeException("MCP {$method} failed: {$msg}");
        }

        return $body['result'] ?? $body;
    }

    /**
     * @param array<string, mixed> $payload
     * @throws GuzzleException
     */
    private function post(array $payload, bool $notification): ResponseInterface
    {
        $headers = [
            'Content-Type' => 'application/json',
            'Accept' => self::ACCEPT,
        ];
        if ($this->token !== null && $this->token !== '') {
            $headers['Authorization'] = 'Bearer ' . $this->token;
        }
        if ($this->sessionId !== null && $this->sessionId !== '') {
            $headers[self::SESSION_HEADER] = $this->sessionId;
        }
        $res = $this->http->post($this->baseUrl, [
            'headers' => $headers,
            'json' => $payload,
            'http_errors' => false,
        ]);
        $code = $res->getStatusCode();
        if ($notification && ($code === 200 || $code === 202)) {
            return $res;
        }
        if ($code < 200 || $code >= 300) {
            throw new \RuntimeException($code . ': ' . (string) $res->getBody());
        }

        return $res;
    }

    /** @param array<string, mixed> $body */
    private function captureSession(ResponseInterface $res, array $body): void
    {
        if ($this->sessionId === null || $this->sessionId === '') {
            $sid = $res->getHeaderLine(self::SESSION_HEADER);
            if (trim($sid) !== '') {
                $this->sessionId = trim($sid);
            }
        }
        if ($this->sessionId === null || $this->sessionId === '') {
            $fromBody = $this->sessionIdFromBody($body);
            if ($fromBody !== null) {
                $this->sessionId = $fromBody;
            }
        }
    }

    /** @param array<string, mixed> $body */
    private function sessionIdFromBody(array $body): ?string
    {
        foreach (['sessionId', 'session_id', 'mcp-session-id'] as $key) {
            if (isset($body[$key]) && is_string($body[$key]) && trim($body[$key]) !== '') {
                return trim($body[$key]);
            }
        }
        $result = $body['result'] ?? null;
        if (is_array($result)) {
            foreach (['sessionId', 'session_id', 'mcp-session-id'] as $key) {
                if (isset($result[$key]) && is_string($result[$key]) && trim($result[$key]) !== '') {
                    return trim($result[$key]);
                }
            }
        }

        return null;
    }

    private function extractPingText(mixed $result): string
    {
        if (is_string($result)) {
            return $result;
        }
        if (!is_array($result)) {
            return (string) json_encode($result);
        }
        $content = $result['content'] ?? null;
        if (is_array($content)) {
            foreach ($content as $item) {
                if (is_array($item) && isset($item['text']) && is_string($item['text'])) {
                    return $item['text'];
                }
            }
        }

        return (string) json_encode($result);
    }
}
