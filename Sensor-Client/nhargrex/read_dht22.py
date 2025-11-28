import time
import adafruit_dht
import board

def read_dht22(dht_device=None, retries=3, delay=2):
    """
    Read DHT22. If dht_device is None, the function will create a local DHT22
    instance on board.D27 and clean it up before returning.
    It will retry on RuntimeError (common with DHT sensors).
    Returns (temperature_f, humidity) or (None, None) on failure.
    """
    created_device = False
    if dht_device is None:
        dht_device = adafruit_dht.DHT22(board.D27)
        created_device = True

    temperature_f = None
    humidity = None

    try:
        for attempt in range(1, retries + 1):
            try:
                temperature_c = dht_device.temperature
                humidity = dht_device.humidity

                if temperature_c is not None and humidity is not None:
                    temperature_f = ((9.0 / 5.0) * temperature_c) + 32.0
                    break

                # If sensor returned None, wait and retry
                if attempt < retries:
                    time.sleep(delay)

            except RuntimeError:
                # DHT sensors often raise RuntimeError on bad reads; retry
                if attempt < retries:
                    time.sleep(delay)
                    continue
                else:
                    temperature_f = None
                    humidity = None

    except Exception:
        # Unexpected exception — return failure
        temperature_f = None
        humidity = None
    finally:
        # If we created the device here, attempt to clean up
        if created_device:
            try:
                if hasattr(dht_device, "exit"):
                    dht_device.exit()
                elif hasattr(dht_device, "close"):
                    dht_device.close()
                else:
                    # best-effort cleanup
                    del dht_device
            except Exception:
                pass

    return (temperature_f, humidity)

def main():
    (temperature_f, humidity) = read_dht22()
    if temperature_f is not None and humidity is not None:
        # Use degree symbol and human-friendly labels
        print(f"{temperature_f:.1f},{humidity:.1f}")
    else:
        print("Failed to retrieve data from DHT22 sensor.")

if __name__ == "__main__":
    main()
