<?php

declare(strict_types=1);

namespace SwissArmyNoife\Sdk;

use GuzzleHttp\Client;
use GuzzleHttp\Exception\GuzzleException;

/** HTTP admin client (sak335-b). */
final class SakClient
{
    private const DEFAULT_HTTP = 'http://127.0.0.1:8787';

    private string $baseUrl;
    private Client $http;

    public function __construct(?string $baseUrl = null, ?Client $http = null)
    {
        $u = ($baseUrl === null || trim($baseUrl) === '') ? self::DEFAULT_HTTP : $baseUrl;
        $this->baseUrl = rtrim($u, '/');
        $this->http = $http ?? new Client(['timeout' => 30.0]);
    }

    public function baseUrl(): string
    {
        return $this->baseUrl;
    }

    /** @return array<string, mixed> */
    public function health(): array
    {
        return $this->getJson('/health');
    }

    /** @return array<string, mixed> */
    public function listModules(): array
    {
        return $this->getJson('/v1/sak/modules');
    }

    /** @return array<string, mixed> */
    public function getModule(string $id): array
    {
        return $this->getJson('/v1/sak/modules/' . rawurlencode($id));
    }

    /** @return array<string, mixed> */
    public function capacity(): array
    {
        return $this->getJson('/v1/sak/capacity');
    }

    /** @return array<string, mixed> */
    public function listWork(): array
    {
        return $this->getJson('/v1/sak/compute/work');
    }

    /** @return array<string, mixed> */
    public function listNodes(): array
    {
        return $this->getJson('/v1/sak/compute/nodes');
    }

    /**
     * @param array<string, mixed> $body
     * @return array<string, mixed>
     */
    public function computeWork(array $body): array
    {
        return $this->postJson('/v1/sak/compute/work', $body);
    }

    /**
     * @param array<string, mixed> $body
     * @return array<string, mixed>
     */
    public function computeNodes(array $body): array
    {
        return $this->postJson('/v1/sak/compute/nodes', $body);
    }

    /**
     * @param array<string, mixed>|null $payload
     * @return array<string, mixed>
     */
    public function enqueueWork(string $kind, ?array $payload = null): array
    {
        return $this->computeWork([
            'action' => 'enqueue',
            'kind' => $kind,
            'payload' => $payload ?? [],
        ]);
    }

    /** @return array<string, mixed> */
    public function claimWork(string $nodeId): array
    {
        return $this->computeWork(['action' => 'claim', 'node_id' => $nodeId]);
    }

    /**
     * @param array<string, mixed>|null $result
     * @return array<string, mixed>
     */
    public function completeWork(string $workId, string $nodeId, ?array $result = null): array
    {
        return $this->computeWork([
            'action' => 'complete',
            'work_id' => $workId,
            'node_id' => $nodeId,
            'result' => $result ?? [],
        ]);
    }

    /** @return array<string, mixed> */
    public function getWork(string $workId): array
    {
        return $this->computeWork(['action' => 'get', 'work_id' => $workId]);
    }

    /** @return array<string, mixed> */
    public function requeueWork(string $workId): array
    {
        return $this->computeWork(['action' => 'requeue', 'work_id' => $workId]);
    }

    /**
     * @param array<string, mixed>|null $filters
     * @return array<string, mixed>
     */
    public function listWorkFiltered(?array $filters = null): array
    {
        return $this->computeWork(array_merge(['action' => 'list'], $filters ?? []));
    }

    /**
     * @param array<string, mixed>|null $filters
     * @return array<string, mixed>
     */
    public function listNodesFiltered(?array $filters = null): array
    {
        return $this->computeNodes(array_merge(['action' => 'list'], $filters ?? []));
    }

    /**
     * @param list<string>|null $caps
     * @return array<string, mixed>
     */
    public function registerNode(
        string $label,
        ?array $caps = null,
        ?string $nodeId = null,
        ?string $sessionId = null,
    ): array {
        $body = ['action' => 'register', 'label' => $label];
        if ($caps !== null) {
            $body['caps'] = $caps;
        }
        if ($nodeId !== null && $nodeId !== '') {
            $body['node_id'] = $nodeId;
        }
        if ($sessionId !== null && $sessionId !== '') {
            $body['session_id'] = $sessionId;
        }

        return $this->computeNodes($body);
    }

    /** @return array<string, mixed> */
    public function heartbeatNode(string $nodeId): array
    {
        return $this->computeNodes(['action' => 'heartbeat', 'node_id' => $nodeId]);
    }

    /**
     * @return array<string, mixed>
     * @throws GuzzleException
     */
    private function getJson(string $path): array
    {
        $res = $this->http->get($this->baseUrl . $path);
        /** @var array<string, mixed> $decoded */
        $decoded = json_decode((string) $res->getBody(), true, 512, JSON_THROW_ON_ERROR);

        return $decoded;
    }

    /**
     * @param array<string, mixed> $payload
     * @return array<string, mixed>
     * @throws GuzzleException
     */
    private function postJson(string $path, array $payload): array
    {
        $res = $this->http->post($this->baseUrl . $path, ['json' => $payload]);
        /** @var array<string, mixed> $decoded */
        $decoded = json_decode((string) $res->getBody(), true, 512, JSON_THROW_ON_ERROR);

        return $decoded;
    }
}
