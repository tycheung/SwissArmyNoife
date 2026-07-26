<?php

declare(strict_types=1);

namespace SwissArmyNoife\Sdk\Tests;

use GuzzleHttp\Client;
use GuzzleHttp\Handler\MockHandler;
use GuzzleHttp\HandlerStack;
use GuzzleHttp\Psr7\Response;
use PHPUnit\Framework\TestCase;
use SwissArmyNoife\Sdk\SakMcpClient;

final class SakMcpClientTest extends TestCase
{
    public function testPingNegotiatesSession(): void
    {
        $mock = new MockHandler([
            new Response(200, ['mcp-session-id' => 'sess-php-1'], '{"jsonrpc":"2.0","id":1,"result":{}}'),
            new Response(202, [], ''),
            new Response(
                200,
                [],
                '{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"pong"}]}}',
            ),
        ]);
        $stack = HandlerStack::create($mock);
        $mcp = new SakMcpClient('http://example.test/mcp', new Client(['handler' => $stack]));
        $this->assertSame('pong', $mcp->ping());
        $this->assertSame('sess-php-1', $mcp->getSessionId());
    }

    public function testToolsListNoAutoInit(): void
    {
        $mock = new MockHandler([
            new Response(200, [], '{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}'),
        ]);
        $stack = HandlerStack::create($mock);
        $mcp = new SakMcpClient('http://example.test/mcp', new Client(['handler' => $stack]));
        $mcp->setAutoInitialize(false);
        $out = $mcp->toolsList();
        $this->assertIsArray($out);
        $this->assertArrayHasKey('tools', $out);
    }

    public function testCatalogList(): void
    {
        $mock = new MockHandler([
            new Response(200, ['mcp-session-id' => 's2'], '{"jsonrpc":"2.0","id":1,"result":{}}'),
            new Response(202, [], ''),
            new Response(200, [], '{"jsonrpc":"2.0","id":2,"result":{"offers":[]}}'),
        ]);
        $stack = HandlerStack::create($mock);
        $mcp = new SakMcpClient('http://example.test/mcp', new Client(['handler' => $stack]));
        $out = $mcp->catalogList();
        $this->assertIsArray($out);
        $this->assertArrayHasKey('offers', $out);
    }
}
