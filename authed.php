<?php

    $out = json_encode($_REQUEST);

    file_put_contents("authed.log", $out, FILE_APPEND);

