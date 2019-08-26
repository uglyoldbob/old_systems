/*
 * common.h
 *
 * Created: 2/14/2017 3:03:12 PM
 *  Author: tepperson
 */ 

#include <stdint.h>

#ifndef COMMON_H_
#define COMMON_H_

void set_prg_rom(uint8_t *dat);
void set_chr_rom(uint8_t *dat);
void set_ram_size(uint16_t size);



#endif /* COMMON_H_ */