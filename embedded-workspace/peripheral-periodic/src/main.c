#include "connection.h"
#include "external_readings.h"
#include "storage.h"
#include "zephyr/bluetooth/bluetooth.h"
#include "zephyr/bluetooth/gatt.h"

static struct bt_conn_cb callbacks = {.connected = connected_callback,
                                      .disconnected = disconnected_callback};

#define SERVICE_UUID 0x183B

static uint8_t periodic_advertising_service_data[] = {
    BT_UUID_16_ENCODE(SERVICE_UUID),
    0x40,
    0,
};

static struct bt_data periodic_advertising_data[] = {
    BT_DATA(BT_DATA_SVC_DATA16, periodic_advertising_service_data,
            ARRAY_SIZE(periodic_advertising_service_data)),
};

static struct bt_data advertising_data[] = {
    // BT_DATA_BYTES(BT_DATA_FLAGS, (BT_LE_AD_GENERAL | BT_LE_AD_NO_BREDR)),
    BT_DATA(BT_DATA_NAME_COMPLETE, CONFIG_BT_DEVICE_NAME,
            sizeof(CONFIG_BT_DEVICE_NAME) - 1),
    // BT_DATA(BT_DATA_SVC_DATA16, periodic_advertising_service_data,
    //         ARRAY_SIZE(periodic_advertising_service_data)),
};

static const struct bt_le_adv_param advertising_parameters[] =
    BT_LE_ADV_PARAM(BT_LE_ADV_OPT_EXT_ADV, BT_GAP_ADV_FAST_INT_MIN_2,
                    BT_GAP_ADV_FAST_INT_MAX_2, NULL);

static int start_advertising(struct bt_le_ext_adv *adv) {
  int err = bt_le_ext_adv_start(adv, BT_LE_EXT_ADV_START_DEFAULT);
  if (err) {
    printk("Failed to start main advertising (err %d)\n", err);
    return err;
  }

  err = bt_le_per_adv_start(adv);
  if (err) {
    printk("Failed to start periodic advertising (err %d)\n", err);
    return err;
  }

  if (!err) {
    k_event_set(&bluetooth_event, ADVERTISING);
  }

  return err;
}

int main(void) {
  int err;

  // This must remain valid memory for the duration of the program. Make it more
  // challenging to move into a function.
  struct bt_le_ext_adv *advertisement_set;

  err = bt_enable(NULL);
  if (err) {
    return err;
  }

  err = bt_le_ext_adv_create(advertising_parameters, NULL, &advertisement_set);
  if (err) {
    return err;
  }

  err = bt_le_ext_adv_set_data(advertisement_set, advertising_data,
                               ARRAY_SIZE(advertising_data), NULL, 0);
  if (err) {
    printk("Failed to set advertising data (err %d)\n", err);
    return err;
  }

  err = bt_le_per_adv_set_param(advertisement_set, BT_LE_PER_ADV_DEFAULT);
  if (err) {
    printk("Failed to set periodic advertising parameters (err %d)\n", err);
    return err;
  }

  err = bt_conn_cb_register(&callbacks);
  if (err) {
    return err;
  }

  err = start_advertising(advertisement_set);
  if (err) {
    printk("Failed to start advertising (err %d)\n", err);
    return err;
  }

  printk("Initializing memory for %d readings\n", 25);
  initialize_storage();

  err = initialize_gpio_readings();
  if (err) {
    printk("could not initialize GPIO readings (err %d)\n", err);
    return err;
  }

  err = initialize_gpio_sampling();
  if (err) {
    printk("could not initialize GPIO sampling (err %d)\n", err);
    return err;
  }

  while (true) {
    k_sleep(K_SECONDS(1));

    // Update periodic advertising data with the latest reading
    const uint8_t reading = sum_stored_readings();

    printk("Updating periodic advertising data with reading: %d\n", reading);
    periodic_advertising_service_data[3] = reading;

    err = bt_le_per_adv_set_data(advertisement_set, periodic_advertising_data,
                                 ARRAY_SIZE(periodic_advertising_data));

    if (err) {
      printk("could not set periodic advertising data (err %d)\n", err);
      return err;
    }
  }
}
