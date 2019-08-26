/*
 * mapper1.h
 *
 * Created: 3/10/2017 12:10:05 PM
 *  Author: tepperson
 */ 


#ifndef MAPPER1_H_
#define MAPPER1_H_


#include "common.h"
uint8_t mapper1_read(uint16_t addr);
void mapper1_write(uint16_t addr, uint8_t val);

void mapper1_set_pgr_rom(uint8_t *data, uint32_t size);


#endif /* MAPPER1_H_ */