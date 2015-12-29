<?php
echo 'rust hello returns: ';
$g = json_decode('{"a": "test"}');
//while(1)
$start = microtime(true);
$n = 1 << 20;
$endless = false;
for ($c = 0; $c < $n || $endless; ++$c)
{
    hello_world($g) == "test" or die("NOPE");
    //break;
}
$end = microtime(true);
echo (($end-$start)) * 1000;
