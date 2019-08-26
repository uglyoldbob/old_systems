/*
 * cartridge.h
 *
 * Created: 2/10/2017 11:05:42 AM
 *  Author: tepperson
 */ 


#ifndef CARTRIDGE_H_
#define CARTRIDGE_H_

#include <stdint.h>

void load_cartridge_image(char *img);
void use_actual_cartridge(void);
uint8_t nes_rom_test(char *name);

#endif /* CARTRIDGE_H_ */