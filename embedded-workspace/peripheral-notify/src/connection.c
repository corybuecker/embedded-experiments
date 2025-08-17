#include "connection.h"

K_EVENT_DEFINE(bluetooth_event);
struct bt_conn *default_connection;

void connected_callback(struct bt_conn *connection, uint8_t err) {
  if (err) {
    return;
  }

  if (default_connection == NULL) {
    default_connection = bt_conn_ref(connection);
  }

  if (default_connection != connection) {
    return;
  }

  printk("client connected to server...");

  k_event_set(&bluetooth_event, CONNECTED);
}

void disconnected_callback(struct bt_conn *connection, uint8_t reason) {
  bt_conn_unref(default_connection);
  default_connection = NULL;

  switch (reason) {
  case BT_HCI_ERR_AUTH_FAIL:
    printk("disconnected: authentication failed (0x%02x)\n", reason);
    break;

  case BT_HCI_ERR_REMOTE_USER_TERM_CONN:
    printk("disconnected: remote user terminated connection (0x%02x)\n",
           reason);
    break;

  case BT_HCI_ERR_REMOTE_LOW_RESOURCES:
    printk("disconnected: remote low resources (0x%02x)\n", reason);
    break;

  case BT_HCI_ERR_REMOTE_POWER_OFF:
    printk("disconnected: remote powered off (0x%02x)\n", reason);
    break;

  case BT_HCI_ERR_CONN_TIMEOUT:
    printk("disconnected: connection timeout (0x%02x)\n", reason);
    break;

  case BT_HCI_ERR_CONN_LIMIT_EXCEEDED:
    printk("disconnected: connection limit exceeded (0x%02x)\n", reason);
    break;

  case BT_HCI_ERR_UNACCEPT_CONN_PARAM:
    printk("disconnected: unacceptable connection parameters (0x%02x)\n",
           reason);
    break;

  case BT_HCI_ERR_UNSUPP_REMOTE_FEATURE:
    printk("disconnected: unsupported remote feature (0x%02x)\n", reason);
    break;

  case BT_HCI_ERR_PAIRING_NOT_SUPPORTED:
    printk("disconnected: pairing not supported (0x%02x)\n", reason);
    break;

  case BT_HCI_ERR_UNSPECIFIED:
    printk("disconnected: unspecified HCI error (0x%02x)\n", reason);
    break;

  default:
    printk("disconnected: reason 0x%02x\n", reason);
    break;
  }

  k_event_set(&bluetooth_event, DISCONNECTED);
}
