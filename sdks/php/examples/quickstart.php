<?php

declare(strict_types=1);

require __DIR__ . '/../vendor/autoload.php';

use SwissArmyNoife\Sdk\SakClient;

$base = getenv('SAK_HTTP') ?: 'http://127.0.0.1:8787';
$sak = new SakClient($base);
echo 'health=' . json_encode($sak->health()) . PHP_EOL;
echo 'modules=' . json_encode($sak->listModules()) . PHP_EOL;
