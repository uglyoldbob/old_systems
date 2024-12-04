#!/bin/bash

wget https://home.uglyoldbob.com/firmware/tang-nano-20k/sipeed_tang_nano_20k.fs
openFPGALoader -b tangnano20k -f ./sipeed_tang_nano_20k.fs
rm ./sipeed_tang_nano_20k.fs

