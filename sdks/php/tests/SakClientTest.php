<?php

declare(strict_types=1);

namespace SwissArmyNoife\Sdk\Tests;

use GuzzleHttp\Client;
use GuzzleHttp\Handler\MockHandler;
use GuzzleHttp\HandlerStack;
use GuzzleHttp\Middleware;
use GuzzleHttp\Psr7\Response;
use PHPUnit\Framework\TestCase;
use SwissArmyNoife\Sdk\SakClient;

final class SakClientTest extends TestCase
{
    public function testBaseUrlStripsSlash(): void
    {
        $c = new SakClient('http://127.0.0.1:8787/');
        $this->assertSame('http://127.0.0.1:8787', $c->baseUrl());
    }

    public function testHealth(): void
    {
        $history = [];
        $c = $this->clientWithResponses(
            [new Response(200, [], '{"ok":true}')],
            $history,
        );
        $this->assertTrue($c->health()['ok']);
        $this->assertSame('/health', $history[0]['request']->getUri()->getPath());
    }

    /** @dataProvider listHelpers */
    public function testListHelpers(string $method, string $path, string $body): void
    {
        $history = [];
        $c = $this->clientWithResponses([new Response(200, [], $body)], $history);
        $out = $c->{$method}();
        $this->assertIsArray($out);
        $this->assertSame($path, $history[0]['request']->getUri()->getPath());
    }

    /** @return array<string, array{0:string,1:string,2:string}> */
    public static function listHelpers(): array
    {
        return [
            'modules' => ['listModules', '/v1/sak/modules', '{"modules":[]}'],
            'work' => ['listWork', '/v1/sak/compute/work', '{"work":[]}'],
            'nodes' => ['listNodes', '/v1/sak/compute/nodes', '{"nodes":[]}'],
            'capacity' => ['capacity', '/v1/sak/capacity', '{"snapshot":{"total_ram_mb":1}}'],
        ];
    }

    public function testEnqueueWork(): void
    {
        $history = [];
        $c = $this->clientWithResponses(
            [new Response(200, [], '{"action":"enqueue","work":{"status":"queued"}}')],
            $history,
        );
        $out = $c->enqueueWork('echo', ['n' => 1]);
        $this->assertSame('enqueue', $out['action']);
        $payload = json_decode((string) $history[0]['request']->getBody(), true);
        $this->assertSame('enqueue', $payload['action']);
        $this->assertSame('echo', $payload['kind']);
    }

    public function testRequeueClaimCompleteGet(): void
    {
        $history = [];
        $responses = array_fill(
            0,
            4,
            new Response(200, [], '{"action":"ok","work":{"id":"w1"}}'),
        );
        $c = $this->clientWithResponses($responses, $history);
        $c->requeueWork('w1');
        $c->claimWork('n1');
        $c->completeWork('w1', 'n1');
        $c->getWork('w1');
        $actions = array_map(
            static fn ($h) => json_decode((string) $h['request']->getBody(), true)['action'],
            $history,
        );
        $this->assertSame(['requeue', 'claim', 'complete', 'get'], $actions);
    }

    /**
     * @param list<Response> $responses
     * @param list<array<string, mixed>> $history
     */
    private function clientWithResponses(array $responses, array &$history): SakClient
    {
        $mock = new MockHandler($responses);
        $stack = HandlerStack::create($mock);
        $stack->push(Middleware::history($history));

        return new SakClient('http://example.test', new Client(['handler' => $stack]));
    }
}
