# Nebula

Nebula is a tool for sending files to nearby devices within the same network.

> "Why?"
>
> Honestly... I was just curious how this kind of thing works.

## How it works

### 1. Discovery

Nebula uses UDP Multicast to discover other devices running Nebula on the same network.

### 2. Interface

Navigate to `http://localhost:35436/page` and you will see a _beatiful_ interface listing discovered
devices, with a button to send them a file.

### 3. File transfer

The file is sent via raw TCP.

## Disclaimer

Nebula is just an educational experiment.

This program is not safe since it lacks cryptography, use it on trusted networks.
