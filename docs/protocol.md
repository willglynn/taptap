# Tigo TAP protocol

This document describes the wired protocol used by Tigo TAPs.

## Background

Tigo makes solar power equipment, including a [system of module-level power electronics](https://www.tigoenergy.com/ts4)
that attach to each solar panel to provide functions like DC optimization, monitoring, and/or rapid shutdown. The full
system involves "smart" devices attached to solar panels, a gateway ("TAP") near those modules, and a cable between that
gateway and a controller.

```text
                       ┌───────────┬───────────┬───────────┬───────────┐
                       │   Solar   │   Solar   │   Solar   │   Solar   │
                       │   Panel   │   Panel   │   Panel   │   Panel   │
┌───┐                  └───┬───┬───┴───┬───┬───┴───┬───┬───┴───┬───┬───┘
│TAP│◀ ─ ─ ─Wireless─ ─ ─ ▶│Opt│◀ ─ ─ ▶│Opt│◀ ─ ─ ▶│Opt│◀ ─ ─ ▶│Opt│
└───┘                      └───┘       └───┘       └───┘       └───┘
  ▲
  │
Wired
  │
  ▼
┌───┐
│CCA│
└───┘
```

The controller collects performance data about the associated solar array. Unfortunately, this information is only
accessible via the manufacturer's cloud services, limiting it to relatively low frequency, low fidelity, and low
availability compared to the hardware itself. Tigo considers this to be a closed system, making both the wired and
wireless components [proprietary protocols](https://en.wikipedia.org/wiki/Proprietary_protocol).

The wired protocol is especially interesting because understanding it would allow passive, external, local monitoring of
this system with no loss of fidelity or unnecessary delay.

```text
┌───┐
│TAP│◀ ─ ─ ─ …
└───┘
  ▲
  │
  │
  ├──────┐
  ▼      ▼
┌───┐  ┌───┐
│CCA│  │O_o│
└───┘  └───┘
```

This approach to monitoring is low risk because it would be completely transparent. The first-party controller remains
in command of all associated equipment at all times, and in fact it cannot even determine whether it is being
monitored. (A full controller replacement would be a much larger project with many more pitfalls.)

The information here is based on the author's observations from a 2024 solar installation consisting of a Tigo CCA, a
Tigo TAP, and 135x Tigo [TS4-A-O](https://www.tigoenergy.com/product/ts4-a-o) DC optimizers. The author developed this
document and the [`taptap` software](https://github.com/willglynn/taptap) for the purpose of interoperability with this
system, and the author is sharing this work so that others may interoperate with their own systems in a similar way.
This information is certainly both incorrect and incomplete, especially with respect to other devices in the product
family, to installations involving multiple TAPs, and to the older "star" systems circa 2018 rather than the newer
"mesh" systems.

This information is provided with no warranty, express or implied.

## Architecture

This system is mostly proprietary but can be described in terms of the [OSI
model](https://en.wikipedia.org/wiki/OSI_model). There are two different networks.

```text
                     Gateway                PV device
                   device (TAP)            (optimizer)
               ┌─────────────────┐     ┌─────────────────┐
       PV   ┌─▶│   Application   │     │   Application   │   Proprietary
  network   │  ├─────────────────┤     ├─────────────────┤    │
            │  │     Network     │     │     Network     │    │
            │  ├─────────────────┤     ├─────────────────┤
            │  │      Link       │     │      Link       │   802.15.4
            │  ├─────────────────┤     ├─────────────────┤    │
            │  │    Physical     │     │    Physical     │    │
            │  └─────────────────┘     └─────────────────┘
            │                  ▲         ▲
            │                  └ ─ ─ ─ ─ ┘
            │  ┌─────────────────┐
  Gateway   └─▶│    Transport    │                           Proprietary
  network      ├─────────────────┤                            │
               │      Link       │                            │
               ├─────────────────┤
               │    Physical     │                           RS-485
               └─────────────────┘
```

Gateways serve as bridges between the gateway network and the PV network. Remote devices are reachable via the PV
network – that is its purpose – but high level gateway functions are accessed indirectly via a command interface which
mostly interacts with the PV network too. The PV network is thus the primary interface for this system, and the gateway
network exists to provide a tunnel from the controller into the PV network.

This document is structured to match the diagram above.

* [Gateway physical layer](#gateway-physical-layer)
* [Gateway link layer](#gateway-link-layer)
* [Gateway transport layer](#gateway-transport-layer)
* [PV physical layer](#pv-physical-layer)
* [PV link layer](#pv-link-layer)
* [PV network layer](#pv-network-layer)
* [PV application layer](#pv-application-layer)

This document is written from the perspective of an external device observing this system, so it begins at the bottom of
the diagram.

## Gateway physical layer

The gateway physical layer provides a shared, byte-oriented, half duplex serial bus. The wired components communicate
over 2-wire [RS-485](https://en.wikipedia.org/wiki/RS-485) at 38400 baud, 8 data bits, no parity, 1 stop bit.

RS-485 is designed for multi-access networks. The manufacturer uses this capability to support connecting multiple
gateways (i.e. TAPs) to the same cable. They intend customers to daisy-chain gateways as needed, they recommend using a
120Ω termination resistor on the far end of bus, and they integrated a termination resistor into the controller device
to ensure that the controller end is always terminated.

Terminating both ends is good practice for RS-485. The bus can be joined from any point along its length, and the bus
can be lengthened as required.

Having said that, while it is good practice to join the bus between the terminators, termination is comparatively
unimportant for receive-only interfaces, and 38400 not a fast baud rate. If your intent is to monitor, positioning
an RS-485 interface (such as a USB RS-485 adapter) a few inches from the controller and connecting it in parallel to the
controller's A and B lines ought to work fine – just take care not to terminate the bus a third time.

## Gateway link layer

The gateway link layer provides unreliable, addressed, checksummed datagram service over the gateway physical layer. Bus
arbitration is implicit, handled by the assumption that gateways reply only when asked, and there are no explicit bus
coordination or flow control mechanisms at this layer.

```text
  Bytes on the bus:  FF 7E 07 92 01 01 49 00 FF EC 66 40 D6 21 7E 08
          Preamble:  FF
    Start of frame:     7E 07
              Body:           92 01 01 49 00 FF EC 66 40 D6 21
      End of frame:                                            7E 08
```

Gateways precede their frames with `FF`, while controllers precede their frames with `00 FF FF`. The implicit bus
arbitration scheme means that it is possible for the controller to transmit request A, give up waiting for a response,
and then begin transmitting request B at the same moment that the gateway attempts to transmit response A. `00` and `FF`
are opposite bus states, so the preamble may be intended for collision detection.

### Escaping

Frames begin with the sequence `7E 07` and are terminated by the sequence `7E 08`. A delimited data format requires some
mechanism to escape the delimiter, and indeed `7E` is used as an escape character, with 7 escape sequences replacing
certain data bytes.

| Escaped | Raw  |
|---------|------|
| `7E 00` | `7E` |
| `7E 01` | `24` |
| `7E 02` | `23` |
| `7E 03` | `25` |
| `7E 04` | `A4` |
| `7E 05` | `A3` |
| `7E 06` | `A5` |

Bytes `23`…`25`, `A3`…`A5` never appear on the bus; they are always substituted with the associated escape sequence
prior to transmission. (Perhaps these bytes pose some backwards compatibility hazard?) Regardless, a receiver must
invert these escape sequences to recover the intended frame contents, since frames certainly _do_ contain these bytes.

### Contents

Gateway frames contain of an address, a type, zero or more payload bytes, and a 16-bit checksum.

```
    Bytes on the bus:  FF 7E 07 92 01 01 49 00 FF 7C DB C2 7E 05 85 7E 08
  Delimiters removed:           92 01 01 49 00 FF 7C DB C2 7E 05 85
   Escaping reversed:           92 01 01 49 00 FF 7C DB C2 A3 85
             Address:           92 01
                Type:                 01 49
             Payload:                       00 FF 7C DB C2
            Checksum:                                      A3 85
```

The address field is a 16-bit quantity. Link layers normally have a source and destination address, but this network
supports only a single controller with no gateway-to-gateway communication, so the address field serves as either "to:
this gateway" or "from: this gateway", indicated by clearing or setting the `80 00` bit respectively. The resulting
addresses (that is, gateway IDs) are therefore 15 bits.

### Checksum

The checksum algorithm is [CRC-16/CCITT](https://en.wikipedia.org/wiki/Cyclic_redundancy_check) with the 0x8408 (0x1021)
polynomial, except that it selects 0x8408 as an initial value just to [avoid being
compatible](https://en.wikipedia.org/wiki/Cyclic_redundancy_check#Obfuscation). This checksum covers the address, type,
and payload together. A detailed checksum calculation for the frame above follows.

```text
                                                        0x8408
    (0x8408 >> 8) ^ CRC_TABLE[(0x8408 & 0xFF) ^ 0x92] = 0x3B57
    (0x3B57 >> 8) ^ CRC_TABLE[(0x3B57 & 0xFF) ^ 0x01] = 0x3788
    (0x3788 >> 8) ^ CRC_TABLE[(0x3788 & 0xFF) ^ 0x01] = 0x19FE
    (0x19FE >> 8) ^ CRC_TABLE[(0x19FE & 0xFF) ^ 0x49] = 0xC12D
    (0xC12D >> 8) ^ CRC_TABLE[(0xC12D & 0xFF) ^ 0x00] = 0xFA26
    (0xFA26 >> 8) ^ CRC_TABLE[(0xFA26 & 0xFF) ^ 0xFF] = 0x4BB6
    (0x4BB6 >> 8) ^ CRC_TABLE[(0x4BB6 & 0xFF) ^ 0x7C] = 0x691D
    (0x691D >> 8) ^ CRC_TABLE[(0x691D & 0xFF) ^ 0xDB] = 0xA353
    (0xA353 >> 8) ^ CRC_TABLE[(0xA353 & 0xFF) ^ 0xC2] = 0x85A3
```

The resulting 0x85A3 checksum is encoded as `A3 85`.

## Gateway transport layer

The gateway transport layer provides reliable exchange of [PV network packets](#pv-network-layer) between the gateways
and the controller, as well as enumeration of attached gateways.

### Receive request

The controller asks for packets a gateway has received by sending a receive request frame (gateway frame type `01 48`).

Receive request payloads consist of a packet number and some unknown fields.

```text
            00 FF FF 7E 07 12 01 01 48 00 01 18 83 04 17 44 7E 08
      Type:                      01 48
   Payload:                            00 01 18 83 04
       ???:                            00 01
  Packet #:                                  18 83
       ???:                                        04
```

Gateways autonomously receive packets over-the-air and record them into an internal buffer. There is no signal that a
gateway has received a PV network packet; the controller must poll each gateway, asking for any packets received after a
given packet number. The gateway will usually respond reporting no packets, but it may also respond with one or more
packets. If the controller successfully received that response, the subsequent request's packet number will be
incremented one or more times; if not, it will still have the earlier packet number and the gateway will retransmit.

```text
Receive request, packet number 0x1883
           00 FF FF 7E 07 12 01 01 48 00 01 18 83 04 17 44 7E 08
  Address:                12 01
     Type:                      01 48
  Payload:                            00 01 18 83 04
 Packet #:                                  18 83

Receive response, 1 packet
           FF 7E 07 92 01 01 49 00 FE 01 83 5A DE 07 00 0A 01 14 63 3A … 7E 08
  Address:          92 01
     Type:                01 49
  Payload:                      00 FE 01 83 5A DE 07 00 0A 01 14 63 3A …

Receive request, packet number 0x1884
           00 FF FF 7E 07 12 01 01 48 00 01 18 84 04 1F 09 7E 08
  Address:                12 01
     Type:                      01 48
  Payload:                            00 01 18 84 04
 Packet #:                                  18 84
```

### Receive response

The answers a receive request frame with a receive response frame (gateway frame type `01 49`). The receive response
frame contains status information followed by zero or more PV network packets.

Most status fields change relatively slowly compared to the frequency of receive polling. Transmitting the full set of
status fields each request/response cycle would be wasteful, so the gateway selectively reveals fields which it believes
are interesting to the controller. It uses one of several status structures depending on which fields it wishes to
disclose.

The largest receive response status structure is `00 E0`, while the smallest is `00 FF`. This 7 byte difference in size
adds up to 1.8 milliseconds, and avoiding this delay is the only reason to introduce this complexity. Gateways
select `00 E0` every 16 receive requests to ensure all fields are regularly synchronized even if some receive responses
are garbled or lost.

The receive response begins with a 16-bit bitfield. The bitfield is read right-to-left. A zero indicates the field is
included. After the indicated optional fields, the Rx response has the low half of the packet number, the slot counter,
and zero or more PV network packets.

```text
Status type: 00 E0
             0    0    E    0
             0000 0000 1110 0000
                               0   Rx buffers used (1 byte)
                              0    Tx buffers free (1 byte)
                             0     ??? A (2 bytes)
                            0      ??? B (2 bytes)
                          0        Packet # high (1 byte)
```

```text
        Payload: 00 E0 04 0E 00 01 02 00 40 FB 21 1B …
    Status type: 00 E0
Rx buffers used:       04
Tx buffers free:          0E
          ??? A:             00 01
          ??? B:                   02 00
  Packet # high:                         40
   Packet # low:                            FB
   Slot counter:                               21 1B
        Packets:                                     …

        Payload: 00 FE 02 FF 21 22 …
    Status type: 00 FE
Rx buffers used:       02
   Packet # low:          FF
   Slot counter:             21 22
        Packets:                   …

        Payload: 00 EE 00 41 01 21 27 …
    Status type: 00 EE
Rx buffers used:       00
  Packet # high:          41
   Packet # low:             01
   Slot counter:                21 27
        Packets:                      …

        Payload: 00 FF 03 21 31 …
    Status type: 00 FF
   Packet # low:       03
   Slot counter:          21 31
        Packets:                …
```

The slot counter seems to be known by remote devices at the [PV link layer](#pv-link-layer), so the slot counter is
documented there.

Each received packet is preceded by a packet header. The author belives this conceptually belongs to the [PV network
layer](#pv-network-layer) and documented it there.

### Command request

The controller can issue a command to the gateway application by sending a command request (gateway frame type `0B 0F`).

```text
                00 FF FF 7E 07 12 01 0B 0F 00 00 00 06 66 00 09 5E 30 30 49 6E 66 6F 0D 3F 8C 7E 08
          Type:                      0B 0F
       Payload:                            00 00 00 06 66 00 09 5E 30 30 49 6E 66 6F 0D
           ???:                            00 00 00
PV packet type:                                     06
    Sequence #:                                        66
  Command data:                                           00 09 5E 30 30 49 6E 66 6F 0D
```

Commands may be answered by the gateway itself immediately via a [command response](#command-response), or the command
may cause a PV network transmission for execution on a remote PV node with the answer delivered sometime later via a
[receive response](#receive-response).

PV packet types and associated command data conceptually belong to the [PV application layer](#pv-application-layer).

The sequence # is chosen by the controller. The value normally increments each command, though the controller avoids
using sequence numbers `FF` or `00`.

### Command response

The gateway responds to a command request frame with a command response frame (gateway frame type `0B 10`).

```text
                 FF 7E 07 92 01 0B 10 00 0D 00 07 66 75 77 7E 08
           Type:                0B 10
        Payload:                      00 0D 00 07 66
            ???:                      00
Tx buffers free:                         0D
            ???:                            00
 PV packet type:                               07
     Sequence #:                                  66
           Data:
```

Some commands return data immediately, with the response data in the command response immediately following the 5-byte
header. This data depends on the command's packet type and is documented at the [PV application
layer](#pv-application-layer).

The author has not observed failed commands. Presumably the controller resends its command request if no command
response is received within some timeout (handling a lost command request), and presumably the gateway buffers command
responses and replays the response to provide idempotence (handling a lost command response).

If the command transmits a packet, the transmitted packet will use a Tx buffer at least until the packet has entered
the gateway's [PV link layer](#pv-link-layer), but more likely until it has been received and acknowledged by a remote
node's PV link layer. (This remote node may or may not be the destination, but as soon as it's received an 802.15.4
acknowledgment, the gateway will not need to retransmit it again and can therefore reclaim that buffer.) Presumably the
controller avoids transmitting when there are no Tx buffers available.

### Ping request

The controller occasionally sends ping requests (gateway frame type `0B 00`).

```text
             00 FF FF 7E 07 12 01 0B 00 01 FE 83 7E 08
       Type:                      0B 00
       Data:                            01
```

The gateway replies with a ping response (gateway frame type `0B 01`).

```text
             FF 7E 07 92 01 0B 01 01 73 10 7E 08
       Type:                0B 01
       Data:                      01
```

### Enumeration start request

The controller sends an enumeration start request (gateway frame type `00 14`) to the broadcast address (`00 00`)
containing unknown data. This is how the controller identifies which gateway(s) are attached to the cable. When the
controller wishes to enumerate gateways, it sends this several times.

```text
                 00 FF FF 7E 07 00 00 00 14 37 7E 01 92 66 12 35 06 1A 7E 08
     Unescaped:                 00 00 00 14 37 24 92 66 12 35 06 1A
       Address:                 00 00
          Type:                       00 14
           ???:                             37 24 92 66
Enumeration ID:                                         12 35
```

The "Enumerate at:" address is a temporary gateway address used for the enumeration process. All gateways receiving an
enumeration start request cease responding at their current gateway ID and begin responding at the requested enumeration
ID instead.

The 32-bit `???` field above appears repeatedly and unchanged during the enumeration process.

<details><summary>Example enumeration sequence</summary>
<p>The enumeration sequence is quite long, but it is illustrative, particularly with respect to the changing addresses.
A full example for a single gateway follows.</p>

```text
Ping request
              00 FF FF 7E 07 12 01 0B 00 01 FE 83 7E 08
    Address:                 12 01
       Type:                       0B 00

Ping response
              FF 7E 07 92 01 0B 01 01 73 10 7E 08
    Address:           92 01
       Type:                 0B 01


Enumeration start request #1
              00 FF FF 7E 07 00 00 00 14 37 7E 01 92 66 12 35 06 1A 7E 08
    Address:                 00 00
       Type:                       00 14

Enumeration start response #1
              FF 7E 07 80 00 00 15 17 E0 7E 08
    Address:           80 00
       Type:                 00 15

Enumeration start request #2
              00 FF FF 7E 07 00 00 00 14 37 7E 01 92 66 12 35 06 1A 7E 08
    Address:                 00 00
       Type:                       00 14

Enumeration start response #2
              FF 7E 07 80 00 00 15 17 E0 7E 08
    Address:           80 00
       Type:                 00 15

Enumeration start request #3
              00 FF FF 7E 07 00 00 00 14 37 7E 01 92 66 12 35 06 1A 7E 08
    Address:                 00 00
       Type:                       00 14

Enumeration start response #3
              FF 7E 07 80 00 00 15 17 E0 7E 08
    Address:           80 00
       Type:                 00 15

Enumeration start request #4
              00 FF FF 7E 07 00 00 00 14 37 7E 01 92 66 12 35 06 1A 7E 08
    Address:                 00 00
       Type:                       00 14

Enumeration start response #4
              FF 7E 07 80 00 00 15 17 E0 7E 08
    Address:           80 00
       Type:                 00 15

Enumeration start request #5
              00 FF FF 7E 07 00 00 00 14 37 7E 01 92 66 12 35 06 1A 7E 08
    Address:                 00 00
       Type:                       00 14

Enumeration start response #5
              FF 7E 07 80 00 00 15 17 E0 7E 08
    Address:           80 00
       Type:                 00 15


Enumeration request #1
              00 FF FF 7E 07 12 35 00 38 5A 72 7E 08
    Address:                 12 35
       Type:                       00 38

Enumeration response
              FF 7E 07 92 35 00 39 04 C0 5B 30 00 02 BE 16 12 35 A7 83 7E 08
    Address:           92 35
       Type:                 00 39

Assign gateway ID request #1
              00 FF FF 7E 07 12 35 00 3C 37 7E 01 92 66 04 C0 5B 30 00 02 BE 16 12 01 58 0B 7E 08
    Address:                 12 35
       Type:                       00 3C

Assign gateway ID response #1
              FF 7E 07 92 35 00 3D 99 08 7E 08
    Address:           92 35
       Type:                 00 3D


Enumeration request #2
              00 FF FF 7E 07 12 35 00 38 5A 72 7E 08
    Address:                 12 35
       Type:                       00 38

Enumeration request #3
              00 FF FF 7E 07 12 35 00 38 5A 72 7E 08
    Address:                 12 35
       Type:                       00 38

Enumeration request #4
              00 FF FF 7E 07 12 35 00 38 5A 72 7E 08
    Address:                 12 35
       Type:                       00 38

Enumeration request #5
              00 FF FF 7E 07 12 35 00 38 5A 72 7E 08
    Address:                 12 35
       Type:                       00 38

Enumeration request #6
              00 FF FF 7E 07 12 35 00 38 5A 72 7E 08
    Address:                 12 35
       Type:                       00 38


Identify request #1
              00 FF FF 7E 07 12 01 00 3A 87 B4 7E 08
    Address:                 12 01
       Type:                       00 3A

Identify response #1
              FF 7E 07 92 01 00 3B 04 C0 5B 30 00 02 BE 16 12 01 E6 A6 7E 08
    Address:           92 01
       Type:                 00 3B

Assign gateway ID request #2
              00 FF FF 7E 07 12 01 00 3C 37 7E 01 92 66 04 C0 5B 30 00 02 BE 16 12 02 DC 60 7E 08
    Address:                 12 01
       Type:                       00 3C

Assign gateway ID response #2
              FF 7E 07 92 01 00 3D 56 ED 7E 08
    Address:           92 01
       Type:                 00 3D

Identify request #2
              00 FF FF 7E 07 12 02 00 3A E3 5B 7E 08
    Address:                 12 02
       Type:                       00 3A

Identify response #2
              FF 7E 07 92 02 00 3B 04 C0 5B 30 00 02 BE 16 12 02 8A 9A 7E 08
    Address:           92 02
       Type:                 00 3B


Unknown 0010
              00 FF FF 7E 07 00 00 00 10 37 7E 01 92 66 C3 27 7E 08
    Address:                 00 00
       Type:                       00 10

Unknown 0011
              FF 7E 07 80 00 00 11 33 A6 7E 08
    Address:           80 00
       Type:                 00 11


Identify request #3
              00 FF FF 7E 07 12 01 00 3A 87 B4 7E 08
    Address:                 12 01
       Type:                       00 3A

Identify response #3
              FF 7E 07 92 01 00 3B 04 C0 5B 30 00 02 BE 16 12 01 E6 A6 7E 08
    Address:           92 01
       Type:                 00 3B

Version request
              00 FF FF 7E 07 12 01 00 0A 04 85 7E 08
    Address:                 12 01
       Type:                       00 0A

Version response
              FF 7E 07 92 01 00 0B 4D 67 61 74 65 20 56 65 72 73 69 6F 6E 20 47 38 2E 35 39 0D 4A 75 6C 20 20 36 20 32 30 32 30 0D 31 36 3A 35 31 3A 35 31 0D 47 57 2D 48 31 35 38 2E 34 2E 33 53 30 2E 31 32 0D 8A E2 7E 08
    Address:           92 01
       Type:                 00 0B


Enumeration end request
              00 FF FF 7E 07 12 01 0E 02 5C 93 7E 08
    Address:                 12 01
       Type:                       0E 02

Enumeration end response
              FF 7E 07 92 01 00 06 06 62 7E 08
    Address:           92 01
       Type:                 00 06


Ping request
              00 FF FF 7E 07 12 01 0B 00 01 FE 83 7E 08
    Address:                 12 01
       Type:                       0B 00

Ping response
              FF 7E 07 92 01 0B 01 01 73 10 7E 08
    Address:           92 01
       Type:                 0B 01
```

</details>

### Enumeration start response

The gateway responds to the enumeration start request by sending an enumeration start response (gateway frame type
`00 15`) from the broadcast address (`80 00`).

```text
              FF 7E 07 80 00 00 15 17 E0 7E 08
    Address:           80 00
       Type:                 00 15
```

### Enumeration request

After starting enumeration, the controller sends an enumeration request (gateway frame type `00 38`) to the enumeration
address (here `12 35`).

```text
               00 FF FF 7E 07 12 35 00 38 5A 72 7E 08
     Address:                 12 35
        Type:                       00 38
```

### Enumeration response

The gateway responds with an enumeration response (gateway frame type `00 39`) from the enumeration address (here
`92 35`). The response contains the [PV link layer hardware address](#pv-link-layer) of the gateway and its current
gateway ID, which was reset to the enumeration ID.

```text
               FF 7E 07 92 35 00 39 04 C0 5B 30 00 02 BE 16 12 35 A7 83 7E 08
     Address:           92 35
        Type:                 00 39
PV long addr:                       04 C0 5B 30 00 02 BE 16
  Gateway ID:                                               12 35
```

The author has exactly one gateway and is unclear about how this works for multiple gateways. Perhaps they take a random
delay and use collision detection such that _a_ gateway will respond, responding with _which_ gateway did so.

### Assign gateway ID request

The controller sends an assign gateway ID request (gateway frame type `00 3C`) to the enumeration ID (here `12 35`).
This request payload includes the hardware address of the intended recipient, likely to disambiguate in the case of
multiple gateways, since at this moment multiple gateways may be configured to use the same gateway ID.

```text
               00 FF FF 7E 07 12 35 00 3C 37 7E 01 92 66 04 C0 5B 30 00 02 BE 16 12 01 58 0B 7E 08
   Unescaped:                 12 35 00 3C 37 24 92 66 04 C0 5B 30 00 02 BE 16 12 01
     Address:                 12 35
        Type:                       00 3C
         ???:                             37 24 92 66
PV long addr:                                         04 C0 5B 30 00 02 BE 16
  Gateway ID:                                                                 12 01
```

Gateway ID assignment is complicated by [unknown 0011](#unknown-00100011).

### Assign gateway ID response

The gateway replies with an assign gateway ID response (gateway frame type `00 3D`) from the enumeration ID (here
`93 35`).

```text
              FF 7E 07 92 35 00 3D 99 08 7E 08
    Address:           92 35
       Type:                 00 3D
```

The gateway then continues enumerating by sending more [enumeration requests](#enumeration-request) until it is
satisfied that no more unidentified gateways are waiting.

### Identify request

The controller sends an identify request (gateway frame type `00 3A`). This is identical to an [enumeration
request](#enumeration-request), except directed at a particular gateway.

```text
              00 FF FF 7E 07 12 01 00 3A 87 B4 7E 08
    Address:                 12 01
       Type:                       00 3A
```

### Identify response

The gateway responds to an identify request by sending an identify response (gateway frame type `00 3B`). This is
identical to an [enumeration response](#enumeration-response), except directed from a particular gateway.

```
               FF 7E 07 92 01 00 3B 04 C0 5B 30 00 02 BE 16 12 01 E6 A6 7E 08
     Address:           92 01
        Type:                 00 3B
PV long addr:                       04 C0 5B 30 00 02 BE 16
  Gateway ID:                                               12 01
```

### Unknown 0010/0011

The controller sends gateway frame type `00 10` to address `00 00` containing unknown data.

```text
              00 FF FF 7E 07 00 00 00 10 37 7E 01 92 66 C3 27 7E 08
  Unescaped:                 00 00 00 10 37 24 92 66 C3 27
    Address:                 00 00
       Type:                       00 10
        ???:                             37 24 92 66
```

The gateway replies with gateway frame type `00 3D` from address `00 00`.

```text

              FF 7E 07 80 00 00 11 33 A6 7E 08
    Address:           80 00
       Type:                 00 11
```

In the example exchange, the controller starts enumerating, assigns gateway ID `12 01` via the enumeration address,
identifies the gateway at `12 01`, then assigns gateway ID `12 02` via address `12 01`, identifies the gateway at
`12 02`, then sends unknown gateway frame type `00 11` to the broadcast address, then identifies the gateway at `12 01`
again. After this, the controller does not address the gateway at `12 02` again.

### Version request

The controller sends a version request (gateway frame type `00 0A`) to a gateway.

```text
              00 FF FF 7E 07 12 01 00 0A 04 85 7E 08
    Address:                 12 01
       Type:                       00 0A
```

### Version response

The gateway responds to a version request by sending a version response (gateway frame type `00 0B`) containing a
version string.

```text
              FF 7E 07 92 01 00 0B 4D 67 61 … 7E 08
    Address:           92 01
       Type:                 00 0B
    Version:                       4D 67 61 …
```

In the author's case, the gateway version string is `"Mgate Version G8.59\rJul  6 2020\r16:51:51\rGW-H158.4.3S0.12\r"`.

### Enumeration end request

The controller ends the enumeration process by sending an enumeration end request (gateway frame type `0E 02`).

```text
              00 FF FF 7E 07 12 01 0E 02 5C 93 7E 08
    Address:                 12 01
       Type:                       0E 02
```

### Enumeration end response

The gateway responds to an enumeration end request with an enumeration end response (gateway frame type `00 06`).

```text
              FF 7E 07 92 01 00 06 06 62 7E 08
    Address:           92 01
       Type:                 00 06
```

## PV physical layer

The PV network uses [IEEE 802.15.4](https://en.wikipedia.org/wiki/IEEE_802.15.4) to exchange data between the gateways
and devices affixed to photovoltaic modules (providing functions like rapid shutdown, monitoring, and/or optimization),
here described as "PV devices", or simply "nodes".

The PV physical layer provides unreliable, low power, short range wireless data exchange. PV devices use the 802.15.4
physical layer over 2.4 GHz at 250 kbps using O-QPSK. In general the PV physical layer is not directly observable from
the gateway network, but some of it is, such as the gateway's radio channel and a received signal strength indicator
(RSSI).

802.15.4 PHYs typically support frames up to 127 bytes long. It is not clear whether this is the precise limit here, but
[node table responses](#node-table) support the idea that the maximum permitted packet body length is >= 124 bytes and
< 134 bytes.

### Transceiver identification

One complication for identifying specific components is that all devices containing a transceiver are designed for
installation outdoors and ought to be potted in epoxy for durability. (Another complication is that the author's modules
are all installed outdoors and doing something useful.) Even so, enough information is available to make some guesses.

According to [a 2015 FCC filing](https://fcc.report/FCC-ID/XMYSILICONAIR2/), Tigo TS4 modules contain the
[NXP JN5148](https://www.nxp.com/products/no-longer-manufactured/32-bit-mcu-and-ieee802-15-4-transceiver:JN5148)
microcontroller, for which [the datasheet](https://www.mouser.com/datasheet/2/302/JN-DS-JN5148-1v7-254806.pdf) provides
detailed capabilities. This chip was discontinued and replaced by the
similar [NXP JN5168](https://www.nxp.com/part/JN5168) ([datasheet](https://www.nxp.com/docs/en/data-sheet/JN516X.pdf),
[802.15.4 stack documentation](https://www.nxp.com/docs/en/user-guide/JN-UG-3024.pdf)). It is likely that the RSSI
values reported from such devices correspond to the graph in the datasheet – roughly linear from 255 meaning -9 dBm to
20 meaning -90 dBm.

According to [a 2022 case
study](https://www.silabs.com/applications/case-studies/bluetooth-low-energy-connectivity-enhances-safety-of-solar-pv-systems),
Tigo TS4 modules contain [Silicon Labs EFR32BG21](https://www.silabs.com/wireless/bluetooth/efr32bg21-series-2-socs)
chips. That particular part number implies using Bluetooth, a suggestion which disagrees with all the other evidence, so
it is more likely that Tigo is using the multiprotocol
[EFR32MG21](https://www.silabs.com/wireless/zigbee/efr32mg21-series-2-socs)
([datasheet](https://www.silabs.com/documents/public/data-sheets/efr32mg21-datasheet.pdf),
[reference](https://www.silabs.com/documents/public/reference-manuals/efr32xg21-rm.pdf)) in ["proprietary" 802.15.4
mode](https://www.silabs.com/documents/public/application-notes/an1365-15-4-over-rail.pdf)
([docs](https://docs.silabs.com/connect-stack/latest/connect-start/)) instead. RSSI values reported from these devices
may use a different (presently unknown) scale.

Circumstantial evidence such as the relative processing capabilities of these NXP and Silicon Labs platforms, the [2016
announcement](https://news.silabs.com/2016-06-29-Silicon-Labs-Multiband-Wireless-Gecko-SoCs-Break-New-Ground-in-the-IoT)
of the Silicon Labs EFR32 platform, the known breaks between Tigo's older "star" and newer "mesh" hardware, and the fact
that [Tigo rolled out "mesh" in
2018](https://www.businesswire.com/news/home/20180718005827/en/Tigo-Releases-State-of-the-art-Wireless-Technology-–-Mesh-–-as-the-New-Solar-Communication-Architecture-for-the-TS4-Platform)
supports the supposition that "star" systems use NXP JN51 chips while "mesh" systems use Silicon Labs EFR32 chips.

## PV link layer

The PV link layer provides reliable, addressed, checksummed, possibly-authenticated-and-maybe-encrypted wireless data
exchange over the PV physical layer. PV devices use the 802.15.4 link layer. In general, the PV link layer is not
directly observable from the gateway network.

### Addressing

The 802.15.4 link layer includes a 64-bit "long address" (EUI-64). This value works like an 802.3 Ethernet MAC address:
it's stamped into a device at the factory, it's stamped onto the packaging outside, etc. All PV network nodes, including
gateways, have fixed 64-bit long addresses built into the hardware.

The 802.15.4 link layer includes a 16-bit "short address". These short addresses are used instead of the long address
in most frame formats. They are dynamically assigned (when the device associates with a PAN coordinator, see
802.15.4-2020 § 6.4.1) and ephemeral. Given that most of the PV network shuts down when it gets dark outside, it's best
to assume that these addresses are not stable. Short addressees leak out in various data structures, presumably places
where the link layer data is known but where other context (like knowledge of which device has this link layer
identifier) is not guaranteed.

Gateways maintain a (persistent?) [node table](#node-table) relating 16-bit PV node IDs to 64-bit long addresses. These
PV node IDs are used instead of either the short or long addresses for most gateway-facing operations. If this was
intended to contain the complexity of the two address formats above, it failed, because it isn't used _everywhere_. The
controller – and any system which wishes to monitor the controller – has to collate all three kinds of addresses.

### Barcodes

Tigo barcodes are an alternate representation of the 802.15.4 long address. `4-9A57A2L` corresponds to
`04:C0:5B:40:00:9A:57:A2`. Tigo's 24-bit vendor prefix is implied (`04:C0:5B`), the `4` indicates the following hex
nibble, the `-` indicates an arbitrary number of zeroes, and all but the final character are the remaining portion of
the address. The final character is a CRC.

```text
  Barcode:          4  -  9A 57 A2 L
   Vendor: 04:C0:5B:
   Nibble:          4
   Zeroes:           0:00:
   Digits:                9A:57:A2
    CRC-4:                         L
```

Tigo selected a CRC-4 using polynomial 0x3 and an initial value of 0x2. (The CRC is similar to [this
one](https://stackoverflow.com/questions/54507106/how-to-classify-following-crc4-implementation), where a Stack Overflow
user notes "it seems unlikely that anyone would use a CRC with so few bytes in a practical application.") The CRC covers
the entire address. The calculation for this example is:

```text
                                   0x2
    CRC_TABLE[0x04 ^ (0x2 << 4)] = 0x6
    CRC_TABLE[0xC0 ^ (0x6 << 4)] = 0x4
    CRC_TABLE[0x5B ^ (0x4 << 4)] = 0xB
    CRC_TABLE[0x40 ^ (0xB << 4)] = 0x6
    CRC_TABLE[0x00 ^ (0x6 << 4)] = 0xD
    CRC_TABLE[0x9A ^ (0xD << 4)] = 0xA
    CRC_TABLE[0x57 ^ (0xA << 4)] = 0xF
    CRC_TABLE[0xA2 ^ (0xF << 4)] = 0x4
```

The resulting CRC nibble is stringified using an alternate set of 16 digits: `GHJKLMNPRSTVWXYZ`. `0x4` is `L`.

### Beacons

The 802.15.4 standard specifies several operating modes and the appropriate modes here involve gateways sending beacons.
These beacons establish a common timebase for all participants, and this shared clock is used to mediate access to the
channel, i.e. who is allowed to transmit when (see 802.15.4-2020 § 6.2). The standard also includes a guaranteed
timeslot mechanism (GTS, see § 6.8) and there is some indication that this is in use. The [gateway radio
configuration](#gateway-radio-configuration) message seems very related.

Gateway beacons likely include the "PV on"/"PV off" signal as part of their [broadcasts](#broadcast), as this would be a
reasonable way to implement the "shut down on command" feature. PV modules also support a separate autonomous "shut down
on controller power failure" feature, which could be triggered by loss of beacons in particular or by disassociation in
general.

### Slot counter

Gateways maintain a slot counter whose current value is known by all network participants. (It may be an 802.15.4-2020
§ 6.2.6.2 absolute slot number, or it may be communicated to listening nodes by some other means.) The slot counter is
read back to the controller via the gateway transport layer [receive responses](#receive-response). Other PV nodes use
the slot counter to synchronize measurements ([power reports](#power-report)) and to agree on the timing of these
reports ([PV configuration](#pv-configuration)). Each slot takes 5 ± 1% milliseconds, varying with the gateway's
internal oscillator.

The slot counter increments autonomously but the values are discontinuous. The slot counter starts at 0x0000 and
increments through 0x2EDF (11999), then jumps to 0x4000 and increments through 0x6EDF (0x4000 + 11999), etc. Put another
way: the top two bits count epochs, while the lower 14 bits count the interval [0, 12000) within each epoch. With 12,000
slots per epoch and 5 ± 1% milliseconds per slot, one epoch is 60 ± 1% seconds.

## PV network layer

The PV network layer provides unreliable, addressed, routed wireless packet service for the PV devices. Note that this
is still an unreliable network, meaning the possibility of PV packet drops requires the controller to explicitly
retransmit packets as needed, even though the reliability of the [gateway transport layer](#gateway-transport-layer)
means the controller can be certain that the gateway received the original transmission.

PV nodes in a "mesh" system have some explicit routing behavior whereby they choose another node as an intermediary to
reach the gateway. This selection is described by [topology reports](#topology-report).

The author belives it is likely that PV network packets share the header returned by the [receive
response](#receive-response) frame, and therefore describes this header as part of the PV network layer. A PV network
packet has the following structure:

```text
         Packet: 07 00 0A 01 14 63 3A …
           Type: 07
     PV node ID:    00 0A
  Short address:          01 14
            DSN:                63
    Data length:                   3A
           Data:                      …58 bytes…
```

The DSN appears to be sourced from the gateway's 802.15.4 MAC state and not from the corresponding PV link layer frame
header. When a PV node aggregates two packets for transmission – say, sending two separate power reports together – the
gateway describes both packets as having the same DSN, even though their sequence numbers would have differed over the
air.

## PV application layer

The author captured PV packets midday to midday on a system using a Tigo Cloud Connect Advanced controller and 135x Tigo
TS4-A-O optimizers. The packet counts below are illustrative of the directions and frequencies despite methodological
flaws.

| Packet type                                                               | Tx   | Rx from gateway | Rx from optimizer |
|---------------------------------------------------------------------------|------|-----------------|-------------------|
| `06` [String request](#string-requestresponse)                            | 2943 | 0               | 0                 |
| `07` [String response](#string-requestresponse)                           | 0    | 0               | 132               |
| `09` [Topology report](#topology-report)                                  | 0    | 0               | 265               |
| `0D` [Gateway radio configuration request](#gateway-radio-configuration)  | 36   | 0               | 0                 |
| `0E` [Gateway radio configuration response](#gateway-radio-configuration) | 0    | 36              | 0                 |
| `13` [PV configuration request](#pv-configuration)                        | 134  | 0               | 0                 |
| `18` [PV configuration response](#pv-configuration)                       | 0    | 0               | 129               |
| `22` [Broadcast](#broadcast)                                              | 58   | 0               | 0                 |
| `23` [Broadcast ack](#broadcast)                                          | 0    | 58              | 0                 |
| `26` [Node table request](#node-table)                                    | 810  | 0               | 0                 |
| `27` [Node table response](#node-table)                                   | 0    | 810             | 0                 |
| `2D` [Long network status request](#network-status)                       | 30   | 0               | 0                 |
| `2E` [Network status request](#network-status)                            | 1401 | 0               | 0                 |
| `2F` [Network status response](#network-status)                           | 0    | 1431            | 0                 |
| `31` [Power report](#power-report)                                        | 0    | 0               | 695057            |
| `41` [Unknown 0x41](#unknown-0x41)                                        | 887  | 0               | 0                 |

### Node table

Gateways maintain a table of mesh nodes mapping 64-bit 802.15.4 long addresses to and from 16-bit PV node IDs. (Note
that these 16-bit PV node IDs are _not_ the same as the 802.15.4 16-bit short addresses.) The gateway itself is also a
mesh node and is implicitly PV node ID `00 01`.

The controller can access this by sending a node table request command (PV packet type `26`).

```text
             00 00
      Index: 00 00
```

The gateway responds with a node table response (PV packet type `27`).

```text
                  00 02 00 0C 04 C0 5B 40 00 A2 34 6F 00 02 04 C0 5B 40 00 A2 34 71 00 03 …
  Starting index: 00 02
       # entries:       00 0C
           Entry:             04 C0 5B 40 00 A2 34 6F 00 02
    Long address:             04 C0 5B 40 00 A2 34 6F
      PV node ID:                                     00 02
           Entry:                                           04 C0 5B 40 00 A2 34 71 00 03
           Entry:                                                                         …
```

Each entry consists of a 64-bit 802.15.4 long address and a 16-bit PV node ID. The table can contain gaps, e.g. this
request for index 0 responded by starting at index 2. (The gateway does not report itself as PV node ID `00 01`, even
though it is.) The end of the table is indicated by the response returning zero entries.

The controller occasionally sends node table requests to other nodes, particularly when they are offline, but they do
not respond even when they are online. This behavior may relate to [unknown 0x41](#unknown-0x41) packets.

### Network status

The controller sends a network status request (PV packet type `2E`) to the gateway (PV node ID `00 01`). This request
has no payload.

The gateway responds with a network status response (PV packet type `2F`). This request describes the number of mesh
nodes, 135 for the system below, three times. These numbers likely diverge when the gateway cannot directly receive
transmissions from every node, but when they are all equal, it is unclear which fields have which meanings.

```
               00 01 01 03 18 00 87 00 87 00 87
   PV node ID: 00 01
          ???:       01
     Counter?:          03 18
  Node count?:                00 87
  Node count?:                      00 87
  Node count?:                            00 87
```

The controller also sends a long network status request (packet type `2D`) containing a node count approximately every
15 minutes, but only overnight when all the nodes except the gateway are offline.

```text
               BA BE 02 03 84 00 87 01
          ???: BA BE 02
     Counter?:          03 84
  Node count?:                00 87
          ???:                      01
```

The gateway responds with packet type `2F`, suggesting that `2D` is an alternate or legacy network status request. It is
also possible that `2D` sets rather than retrieves the second unknown field in the response, given that the request
above produced the response below.

```text
               01 03 84 00 87 00 87 00 87
          ???: 01
      Counter:    03 84
  Node count?:          00 87
  Node count?:                00 87
  Node count?:                      00 87
```

### Gateway radio configuration

The controller can retrieve some gateway radio configuration data by sending a gateway radio configuration request (PV
packet type `0D`).

```text
       00 01
  ???: 00 01
```

The gateway responds with configuration data (PV packet type `0E`):

```text
                  00 15 24 F6 18 04 02 01 00 00 00 00 1C 01 5A (…16 bytes…) 00 00 02 00 3C 00
             ???: 00
   Radio channel:    15
          PAN ID:       24 F6
            CAP?:             18
            CFP?:                04
            BOP?:                   02
            IAP?:                      01
             ???:                         00 00 00 00 1C 01 5A
  Encryption key:                                              (…16 bytes…)
             ???:                                                           00 00 02 00 3C 00
```

The positively identified fields concern the [PV link layer](#pv-link-layer) or [PV physical layer](#pv-physical-layer),
but without more precise measurements from the radio side, it's difficult to be more specific.

802.15.4 devices communicate as part of a "Personal Area Network", or PAN, and all link-layer frames include the PAN ID.
Gateways ought to perform standard PAN ID conflict resolution per 802.15.4-2020 § 6.3.2, meaning this value is subject
to change without notice.

The encryption key matches the length expected for AES-128. It is unclear exactly how this is used, but the obvious
candidates are MIC-128 or ENC-MIC-128 per 802.15.4-2020 § 9.3.

The CAP?, CFP?, BOP?, and IAP? values could plausibly describe the structure of the gateway's 802.15.4 superframe,
specifying the durations of a contention access period, a contention free period, a beacon-only period, and an inactive
period per 802.15.4-2020 § 6.2.1. This hypothesis could be supported or falsified by radio observations which the author
has not yet conducted.

### PV configuration

The controller sends modules a PV configuration request (PV packet type `13`). This is done at low frequency, i.e. less
than once per module per day.

This packet type assigns the power report cycle's period and phase with reference to the gateway's slot counter. The
period `0F A0` is 4000 in decimal, and 4000 slots at 5±1% milliseconds per slot corresponds to a measurement interval of
20±1% seconds. Each module is assigned a different reporting phase, spreading the load and minimizing retransmissions at
the [PV link layer](#pv-link-layer).

```text
            00 39 03 00 31 02 0F A0 09 7D 00 09 02 00 00 00 00 00 30 02 00 00 00 00
PV node ID: 00 39
       ???:       03 00
      Type:             31
       ???:                02
    Period:                   0F A0
     Phase:                         09 7D
       ???:                               00 09 02 00 00 00 00 00 30 02 00 00 00 00
```

The PV node returns a configuration request (PV packet type `18`). The response includes both radio parameters (like the
802.15.4 PAN ID and radio channel) and reporting parameters (echoed from the request). Both groups are separately
duplicated, possibly indicating an alternate or backup configuration for each.

```text
              0F 24 F6 15 6C 00 (5 bytes) 03 00 30 00 00 00 00 00 31 0F A0 09 7D 00 09 00 00 00 00 (19 bytes)
         ???: 0F
      PAN ID:    24 F6
     Channel:          15
         ???:             6C 00
  Alternate?:                   (repeat)
         ???:                             03 00 30 00 00 00 00 00
        Type:                                                     31
      Period:                                                        0F A0
       Phase:                                                              09 7D
         ???:                                                                    00 09 00 00 00 00
  Alternate?:                                                                                      (repeat)
```

### Broadcast

The controller sends broadcast data (packet type `22`) to PV node ID `00 00` (which is presumably a PV node broadcast
address).

```text
          00 01
     ???: 00 0
  PV off:     1
```

When "PV off" is maintained for extended periods, the controller de-asserts and re-asserts this a few milliseconds
later at one hour intervals. (Why? Does this have an effect over-the-air?)

The gateway replies (packet type `23`) to acknowledge.

When PV off is not asserted, the controller occasionally sends empty packets of type `22` to the broadcast address and
to the gateway.

### Topology report

PV devices send topology reports (PV packet type `09`).

```text
                 00 02 00 58 00 01 00 02 04 C0 5B 40 00 9A 57 BB 9F 01 E9 E1 08 95 27
  Short address: 00 02
     PV node ID:       00 58
       Next-hop:             00 01
            ???:                   00 02
   Long address:                         04 C0 5B 40 00 9A 57 BB
           RSSI:                                                 9F
            ???:                                                    01 E9 E1 08 95 27
```

These messages are the first received by the gateway as a node joins the network at power-on, but they are also received
unsolicited during normal operation.

### Power report

PV devices periodically measure their operating parameters and send power reports (PV packet type `31`).

```text
                    2B 61 58 FF 03 21 58 81 00 6E 8F A0 7E
        Voltage in: 2B 6                                     0x2B6 = 694 * 0.05V = 34.7V
       Voltage out:     1 58                                 0x158 = 344 * 0.10V = 34.4V
  DC-DC duty cycle:          FF                              0xFF = 255 = 100%
        Current in:             03 2                         0x032 = 50 * 0.005A = 0.025A
       Temperature:                 1 58                     0x158 = 344 * 0.1ºC = 34.4ºC
               ???:                      81 00 6E
      Slot counter:                               8F A0      timestamp referenced to gateway
              RSSI:                                     7E   0x7E = 126
```

PV devices take measurements simultaneously at intervals synchronized to the gateway's slot counter. They transmit these
measurements asynchronously, often with significant latency, including delaying them more than one measurement window.
Sometimes PV devices bundle multiple power reports together into a single transmission. The slot counter field allows
the receiver to temporally correlate the power report with the gateway's slot counter when the measurement was
performed.

### String request/response

PV modules support string-based requests (PV packet type `06`) and responses (PV packet type `07`). Requests are
preceded by a PV node ID. Requests appear to be terminated by ASCII carriage return (0x0D), while responses may or may
not have such termination. The packet data for known string requests/responses are described below, using C-style string
notation.

```text
            00 3D 5E 30 30 54 65 73 74 73 0D
PV node ID: 00 2D
    String:       5E 30 30 54 65 73 74 73 0D    "^00Tests\r"
```

#### Info

Request: `"^00Info\r"`

Response: `"!Info 0000 15 0000 0981 00 0000 0000 FF 00 0000 0FFF 000 2"`

#### MPPT

Request: `"^00Mppt_1.1\r"`

Response: none
s

#### Tests

Request: `"^00Tests\r"`

Response: `"!Tests 0 0 2 0 00 0000 00\r"`

#### Smrt

Request: `"^00Smrt\r"`

Response: `"!Smrt 0FFF 0000 0FFF 0008 00C8 00C8 000A 0154 0154 S 00 01"`

#### Version

Request: `"^00Version\r"`

Response: `"Mnode Version K8.0120 (2D)\r"`

#### `w`

Request: `#00w255\r`

Response: none

### Unknown 0x41

The controller sends an unknown packet (packet type `41`) to the gateway and to other PV nodes.

```text
             00 00 00 01 00 AA 00 00 00 00 04 05 0F E8 08 95 09 FD 8F F6 05 C7
        ???: 00 00
PV node ID?:       00 01
        ???:             00 AA 00 00 00 00 04 05 0F E8 08 95 09 FD 8F F6 05 C7
```

An example sent to another node:

```text
             00 00 00 02 00 AA 00 00 00 00 04 05 0F E8 08 95 09 FD 8F F6 05 C7
        ???: 00 00
PV node ID?:       00 02
        ???:             00 AA 00 00 00 00 04 05 0F E8 08 95 09 FD 8F F6 05 C7
```

These packets do not elicit replies.

The timing is unusual: the controller sends these to the gateway when overnight it sees no other devices, it sends them
to the first couple devices immediately after sunrise when they come online, and it sends them to the last couple
devices immediately after sunset when they've already gone offline.
