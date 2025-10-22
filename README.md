# `taptap`

This project implements the [Tigo TAP](https://www.tigoenergy.com/product/tigo-access-point) protocol, especially for
the purpose of monitoring a Tigo TAP and the associated solar array using the TAP's communication cable. This allows
100% local offline data collection.

The TAP protocol is described at [`docs/protocol.md`](https://github.com/willglynn/taptap/blob/master/docs/protocol.md).
This system uses two networks, a wired "gateway network" and a wireless "PV network":

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

## Connecting

The gateway network runs over RS-485 and can support more than two connections. An owner may therefore connect a USB
RS-485 adapter, or an RS-485 hat, or any other RS-485 interface without interrupting communication.

The gateway network supports a single controller. Most owners use a Tigo Cloud Connect Advanced (CCA), but there are
alternatives, including older Tigo products and similar controllers embedded in GoodWe inverters. `taptap` can observe
the controller's communication, without ever transmitting anything; as far as the other components are concerned, it
does not exist. This allows owners to gather real-time information from their own hardware without going through Tigo's
cloud platform and without modifying the controller, their TAPs, or any other hardware in any way.

<details>
<summary>Placement considerations</summary>
<p>This system uses a 4-wire bus: ground (– or ⏚), power (+), A, and B. These wires are intended to run from the
controller to a TAP, and possibly to another TAP, and so on. The A and B wires carry RS-485 signals. Tigo recommends
putting a 120Ω resistor on the last TAP's A and B wires to terminate the far end of the bus, and they built a 120Ω
resistor into the controller to terminate the near end of the bus.</p>

<p>If you are adding a monitoring device to an existing install, it would be best to move the controller's A and B wires
to the monitoring device, and then to run new wires from there to the controller. Having said that, it should be fine to
connect short wires from the controller's A and B terminals to the monitoring device, especially if you plan never to
transmit. (Your monitoring device may also have a "ground" or "reference" terminal, which should go to the controller's
gateway ⏚ ground.) In either case, make sure the RS-485 interface you're adding does not include a third termination
resistor. The bus should always be terminated at the controller and at the furthest away TAP.</p>

```text
┌─────────────────────────────────────┐      ┌────────────────────────────┐
│                 CCA                 │      │            TAP             │
│                                     │      │                            │
│ AUX  RS485-1  GATEWAY  RS485-2 POWER│      │                    ┌~┐     │
│┌─┬─┐ ┌─┬─┬─┐ ┌─┬─┬─┬─┐ ┌─┬─┬─┐ ┌─┬─┐│      │   ┌─┬─┬─┬─┐   ┌─┬─┬│┬│┐    │
││/│_│ │-│B│A│ │-│+│B│A│ │-│B│A│ │-│+││      │   │-│+│B│A│   │-│+│B│A│    │
│└─┴─┘ └─┴─┴─┘ └│┴│┴│┴│┘ └─┴─┴─┘ └─┴─┘│      │   └│┴│┴│┴│┘   └─┴─┴─┴─┘    │
└───────────────│─│─│─│───────────────┘      └────│─│─│─│─────────────────┘
                │ │ │ │                           │ │ │ │
                │ │ │ ┃───────────────────────────│─│─│─┘
                │ │ ┃─┃───────────────────────────│─│─┘
                │ └─┃─┃───────────────────────────│─┘
                ┃───┃─┃───────────────────────────┘
                ┗━┓ ┃ ┃
              ┌───┃─┃─┃───┐
              │  ┌┃┬┃┬┃┐  │
              │  │-│B│A│  │
              │  └─┴─┴─┘  │
              │  Monitor  │
              └───────────┘
```

</details>

<details>
<summary>Future work: controller-less operation</summary>
<p>In the absence of another controller, <code>taptap</code> could request PV packets from the gateway(s) itself. The
gateway and PV modules appear to function autonomously after configuration, so for a fully commissioned system,
receiving PV packets from the gateway without ever transmitting anything to the modules would likely be sufficient for
monitoring.</p>
</details>

<details>
<summary>Software-based connection method for owners with <code>root</code> access on their controller</summary>
<p>Some owners have <code>root</code> access on their controller. These owners could install
<a href="https://github.com/willglynn/tcpserial_hook"><code>tcpserial_hook</code></a> on their controller to make the
serial data available over the LAN, including to <code>taptap</code>, without physically adding another RS-485
interface.</p>
<p>This method has several disadvantages: it requires <code>root</code> access, it requires (reversibly) modifying the
files on the controller, it might stop working in future firmware updates, it only works when the controller is working,
etc. It is a fast way to get started for some users, but consider wiring in a separate RS-485 interface instead.</p>
</details>

## Project structure

`taptap` consists of a library and an executable. The executable is a CLI:

```console
% taptap
Usage: taptap <COMMAND> <OPTION>

Commands:
  observe               Observe the system, extracting data as it runs
  list-serial-ports     List `--serial` ports
  peek-bytes            Peek at the raw data flowing at the gateway physical layer
  peek-frames           Peek at the assembled frames at the gateway link layer
  peek-activity         Peek at the gateway transport and PV application layer activity
  help                  Print this message or the help of the given subcommand(s)
  

Options:
  --serial              The name of the serial port (try `taptap list-serial-ports`) of the Modbus-to-serial device (mutually exclusive to --tcp)
  --tcp                 The IP or hostname of the device which is providing Modbus-over-TCP service (mutually exclusive to --serial)
  --reconnect_timeout   The time after which connection is re-established if no data is received in seconds (default is 0s, i.e. no timeout)
  --reconnect-retry     The number of times to retry reconnecting before giving up (default is 0, i.e. infinite retries)
  --reconnect-delay     The delay between reconnect attempts in seconds (default is 5s)
  --port                If --tcp is specified, the port to which to connect (default is 502)
  --keepalive_idle      If --tcp is specified, the idle time in seconds before TCP keepalive probes are sent (default is 30s)
  --keepalive_interval  If --tcp is specified, the interval between individual TCP keepalive probes in seconds (default is 10s)
  --keepalive_count     If --tcp is specified, the number of unacknowledged TCP keepalive probes before the connection is considered dead (default is 5)
  -h, --help            Print help
  -V, --version         Print version

% taptap observe --tcp 172.21.3.44
{"gateway":{"id":4609},"node":{"id":116},"timestamp":"2024-08-24T09:16:41.686961-05:00","voltage_in":30.6,"voltage_out":30.2,"current":6.94,"dc_dc_duty_cycle":1.0,"temperature":26.8,"rssi":132}
{"gateway":{"id":4609},"node":{"id":116},"timestamp":"2024-08-24T09:17:01.691683-05:00","voltage_in":30.75,"voltage_out":30.4,"current":6.895,"dc_dc_duty_cycle":1.0,"temperature":26.8,"rssi":132}
{"gateway":{"id":4609},"node":{"id":82},"timestamp":"2024-08-24T09:16:41.686961-05:00","voltage_in":30.55,"voltage_out":30.2,"current":6.845,"dc_dc_duty_cycle":1.0,"temperature":29.3,"rssi":147}
{"gateway":{"id":4609},"node":{"id":82},"timestamp":"2024-08-24T09:17:01.691683-05:00","voltage_in":30.95,"voltage_out":30.6,"current":6.765,"dc_dc_duty_cycle":1.0,"temperature":29.3,"rssi":147}
{"gateway":{"id":4609},"node":{"id":19},"timestamp":"2024-08-24T09:16:41.686961-05:00","voltage_in":30.35,"voltage_out":29.9,"current":6.865,"dc_dc_duty_cycle":1.0,"temperature":28.7,"rssi":147}
{"gateway":{"id":4609},"node":{"id":19},"timestamp":"2024-08-24T09:17:01.691683-05:00","voltage_in":29.85,"voltage_out":29.4,"current":7.005,"dc_dc_duty_cycle":1.0,"temperature":28.7,"rssi":147}
{"gateway":{"id":4609},"node":{"id":121},"timestamp":"2024-08-24T09:16:41.686961-05:00","voltage_in":29.8,"voltage_out":21.9,"current":5.25,"dc_dc_duty_cycle":0.7607843137254902,"temperature":29.8,"rssi":120}
{"gateway":{"id":4609},"node":{"id":121},"timestamp":"2024-08-24T09:17:01.691683-05:00","voltage_in":30.55,"voltage_out":22.8,"current":5.3,"dc_dc_duty_cycle":0.7725490196078432,"temperature":29.8,"rssi":120}
```

As of this initial version, the `observe` subcommand emits `taptap::observer::Event`s to standard output as JSON rather
than emitting metrics for InfluxDB or Prometheus, and it does not persist its own state, meaning the gateway and nodes
are identified by their internal IDs rather than by barcode. These are the next two features to add.
