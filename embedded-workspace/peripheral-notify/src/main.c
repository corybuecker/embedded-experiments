#include "zephyr/bluetooth/bluetooth.h"
#include "zephyr/bluetooth/gatt.h"

#include "connection.h"
#include "external_readings.h"
#include "storage.h"

static struct bt_conn_cb callbacks = {.connected = connected_callback,
                                      .disconnected = disconnected_callback};

#define SERVICE_UUID 0x183B
#define CHARACTERISTIC_UUID 0x183C

static const struct bt_uuid_16 service_uuid = BT_UUID_INIT_16(SERVICE_UUID);
static struct bt_data advertising_data[] = {
    BT_DATA_BYTES(BT_DATA_FLAGS, (BT_LE_AD_GENERAL | BT_LE_AD_NO_BREDR)),
    BT_DATA_BYTES(BT_DATA_UUID16_ALL, BT_UUID_16_ENCODE(SERVICE_UUID)),
    BT_DATA(BT_DATA_NAME_COMPLETE, CONFIG_BT_DEVICE_NAME,
            sizeof(CONFIG_BT_DEVICE_NAME) - 1),
};

static void notify_ccc_changed(const struct bt_gatt_attr *attr,
                               uint16_t value) {};

// // These variables HAVE to remain valid memory for the duration of the
// program.
static const struct bt_uuid_16 characteristic_uuid =
    BT_UUID_INIT_16(CHARACTERISTIC_UUID);

struct bt_gatt_attr service_gatt_attributes[] = {
    BT_GATT_PRIMARY_SERVICE(&service_uuid),
    BT_GATT_CHARACTERISTIC(&characteristic_uuid.uuid, BT_GATT_CHRC_NOTIFY,
                           BT_GATT_PERM_NONE, NULL, NULL, NULL),
    BT_GATT_CCC(notify_ccc_changed, BT_GATT_PERM_READ | BT_GATT_PERM_WRITE)};

struct bt_gatt_service service = BT_GATT_SERVICE(service_gatt_attributes);

static int start_advertising(struct bt_le_ext_adv *adv) {
  const int err = bt_le_ext_adv_start(adv, BT_LE_EXT_ADV_START_DEFAULT);

  if (!err) {
    k_event_set(&bluetooth_event, ADVERTISING);
  }

  return err;
}

K_THREAD_STACK_DEFINE(my_stack_area, 4096);
struct k_thread my_thread_data;

void my_thread_entry_point(void *p1, void *p2, void *p3) {
  while (true) {
    k_sleep(K_MSEC(250));

    const uint8_t reading = sum_stored_readings();

    if (default_connection != NULL) {
      if (bt_gatt_is_subscribed(default_connection, &service_gatt_attributes[1],
                                BT_GATT_CCC_NOTIFY)) {
        const int err =
            bt_gatt_notify(default_connection, &service_gatt_attributes[1],
                           &reading, sizeof(reading));
        if (err) {
          printk("bt_gatt_notify failed (err %d)\n", err);
        }
      }
    }
  }
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

  err = bt_gatt_service_register(&service);
  if (err) {
    return err;
  }

  err = bt_le_ext_adv_create(BT_LE_EXT_ADV_CONN, NULL, &advertisement_set);
  if (err) {
    return err;
  }

  err = bt_le_ext_adv_set_data(advertisement_set, advertising_data,
                               ARRAY_SIZE(advertising_data), NULL, 0);
  if (err) {
    return err;
  }

  err = bt_conn_cb_register(&callbacks);
  if (err) {
    return err;
  }

  err = start_advertising(advertisement_set);
  if (err) {
    printk("could not start advertising");
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

  k_thread_create(&my_thread_data, my_stack_area,
                  K_THREAD_STACK_SIZEOF(my_stack_area), my_thread_entry_point,
                  NULL, NULL, NULL, 8, 0, K_NO_WAIT);

  while (true) {
    printk("Waiting for Bluetooth events...\n");
    k_event_wait(&bluetooth_event, DISCONNECTED, false, K_FOREVER);

    err = start_advertising(advertisement_set);
    if (err) {
      k_event_set(&bluetooth_event, DISCONNECTED);
      k_sleep(K_SECONDS(1));
    }

    printk("Advertising started successfully!\n");
  }
}
