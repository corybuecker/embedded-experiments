#ifndef NOTIFIER_GPIO_READINGS_H
#define NOTIFIER_GPIO_READINGS_H

#include "storage.h"
#include "zephyr/drivers/gpio.h"

extern const struct gpio_dt_spec button;

typedef struct external_reading {
  uint8_t value;
  struct external_reading *next;
} external_reading_t;

int initialize_gpio_readings();
int initialize_gpio_sampling();

#endif  // NOTIFIER_GPIO_READINGS_H
