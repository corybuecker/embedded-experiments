//
// Created by Cory Buecker on 8/10/25.
//

#ifndef NOTIIFER_CONNECTION_H
#define NOTIIFER_CONNECTION_H

#include "zephyr/bluetooth/conn.h"
#include "zephyr/sys/util_macro.h"

// TODO: document how the bitmask works with the event system.
enum ConnectionState {
  DISCONNECTED = BIT(0),
  CONNECTED = BIT(1),
  ADVERTISING = BIT(2),
};

extern struct bt_conn *default_connection;
extern struct k_event bluetooth_event;

void connected_callback(struct bt_conn *connection, uint8_t err);
void disconnected_callback(struct bt_conn *connection, uint8_t reason);
#endif // NOTIIFER_CONNECTION_H
