/*
 * mapper0.h
 *
 * Created: 2/14/2017 2:48:24 PM
 *  Author: tepperson
 */ 


#ifndef MAPPER0_H_
#define MAPPER0_H_

#include <stdint.h>

#include "common.h"
uint8_t mapper0_read(uint16_t addr);
void mapper0_write(uint16_t addr, uint8_t val);

void mapper0_set_pgr_rom(uint8_t *data, uint32_t size);


#endif /* MAPPER0_H_ */