//
// Created by Cory Buecker on 8/12/25.
//

#ifndef NOTIFIER_STORAGE_H
#define NOTIFIER_STORAGE_H

#include "external_readings.h"
#include "zephyr/kernel.h"

#define MAXIMUM_STORED_READINGS 25
#define READING_SIZE_IN_BYTES sizeof(external_reading_t)
#define REQUIRED_MEMORY (READING_SIZE_IN_BYTES * MAXIMUM_STORED_READINGS * 3)

void initialize_storage();

void store_reading(uint8_t value);
uint8_t sum_stored_readings();

#endif // NOTIFIER_STORAGE_H
