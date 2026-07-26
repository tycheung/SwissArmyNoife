<?php

declare(strict_types=1);

namespace SwissArmyNoife\Sdk\Tests;

use PHPUnit\Framework\TestCase;
use SwissArmyNoife\Sdk\SdkInfo;

final class ScaffoldTest extends TestCase
{
    public function testSdkInfoName(): void
    {
        $this->assertSame('swissarmynoife/sdk', SdkInfo::NAME);
    }
}
