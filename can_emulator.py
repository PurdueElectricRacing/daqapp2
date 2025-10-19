import os
import time
import random
import threading
import subprocess
import warnings

import can
import cantools

DBC_PATH = "per_dbc_VCAN.dbc"   # path to your DBC
CHANNEL = "vcan0"          # virtual CAN channel
BITRATE = 500000


def setup_vcan():
    """Create a virtual CAN interface if it doesn't exist."""
    os.system("sudo modprobe vcan")
    os.system(f"sudo ip link add dev {CHANNEL} type vcan || true")
    os.system(f"sudo ip link set up {CHANNEL}")


def start_slcanpty():
    """Expose the virtual interface as a PTY SLCAN device."""
    print("[slcanpty] Starting...")
    proc = subprocess.Popen(["slcanpty", CHANNEL],
                            stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE,
                            text=True)

    # Wait for PTY to appear
    time.sleep(0.5)
    pty_path = None
    if proc.stderr:
        for line in proc.stderr.readlines():
            if "/dev/pts/" in line:
                pty_path = line.strip()
                break

    if not pty_path:
        pty_path = "/dev/pts/unknown"
    print(f"[slcanpty] PTY device: {pty_path}")
    return proc, pty_path


def send_random_frames(dbc_path, bus):
    """Continuously send randomized CAN frames using a DBC file."""
    db = cantools.database.load_file(dbc_path)
    messages = db.messages
    print(f"[sim] Loaded {len(messages)} messages from {dbc_path}")

    while True:
        msg = random.choice(messages)
        signal_values = {}

        for sig in msg.signals:
            # Compute a safe min/max range
            if sig.minimum is not None and sig.maximum is not None:
                low, high = sig.minimum, sig.maximum
            else:
                # Derive from bit length and signedness
                if sig.is_signed:
                    low = -(2 ** (sig.length - 1))
                    high = (2 ** (sig.length - 1)) - 1
                else:
                    low = 0
                    high = (2 ** sig.length) - 1

            # Generate a value safely within that range
            if sig.is_float:
                val = random.uniform(low, high * 0.9999)
            elif sig.is_signed:
                val = random.randint(int(low), int(high))
            else:
                val = random.randint(int(low), int(high))

            signal_values[sig.name] = val

        try:
            data = msg.encode(signal_values)
            frame = can.Message(
                arbitration_id=msg.frame_id,
                data=data,
                is_extended_id=msg.is_extended_frame,
            )
            bus.send(frame)
            print(f"[sim] {msg.name:<20} ID=0x{msg.frame_id:03X} {signal_values}")
        except Exception as e:
            print(f"[sim] Send failed for {msg.name}: {e}")

        time.sleep(0.1)


def main():
    setup_vcan()

    print("[sim] Opening virtual CAN bus...")
    bus = can.Bus(channel=CHANNEL, interface="socketcan")

    slcan_proc, pty_path = start_slcanpty()

    # Start traffic generator
    sender = threading.Thread(target=send_random_frames, args=(DBC_PATH, bus), daemon=True)
    sender.start()

    print(f"[sim] Running. Connect to {pty_path} in your Rust app.")
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        print("\n[sim] Shutting down...")
    finally:
        slcan_proc.terminate()
        os.system(f"sudo ip link del {CHANNEL}")


if __name__ == "__main__":
    main()
