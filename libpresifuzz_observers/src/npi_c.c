// SPDX-FileCopyrightText: 2022 Intel Corporation
//
// SPDX-License-Identifier: Apache-2.0

/* --------------------------------------------------------------------------------
 * Description:
 * NPI Coverage Model using C/C++
 * This example demonstrates
 * 1. Open a coverage databse.
 * 2. Merge test.
 * 3. Traverse instance from top
 * 4. Copy to map selected metric
 * -------------------------------------------------------------------------------- */

#include "stdio.h"
#ifndef DUMMY_LIB
#include "npi.h"
#include "npi_cov.h"
#else
typedef void* npiCovHandle;
#endif

#include <stdio.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <sys/stat.h>        /* For mode constants */
#include <fcntl.h>           /* For O_* constants */
#include <unistd.h>
#include <sys/types.h>
#include <string>
#include <ctime>

#include "stdint.h"

#ifdef __cplusplus
extern "C" {
#endif

  typedef struct {
    uint32_t* map;
    unsigned write_byte_index;
    unsigned write_bit_index;
    unsigned type;
    unsigned size;
    uint32_t coverable;
    uint32_t covered;
    char*    filter;
  }CoverageMap;
  
  void instance_map_size( npiCovHandle scope, npiCovHandle test, CoverageMap* cov_map);
  void compute_size( npiCovHandle inst, npiCovHandle test, CoverageMap* cov_map);
  size_t compute_map_size(npiCovHandle db, unsigned coverage_type, char* filter);

  void dump_instance_coverage( npiCovHandle scope, npiCovHandle test, CoverageMap* cov_map);
  npiCovHandle vdb_cov_init(const char* vdb_file_path);
  void vdb_cov_end(npiCovHandle db);
  void compute_score( npiCovHandle inst, npiCovHandle test, CoverageMap* cov_map);
  void update_cov_map(npiCovHandle db, uint32_t* map, unsigned map_size, unsigned coverage_type, char* filter);
  void vdb_init();
  
  void vdb_init() {
#ifdef DUMMY_LIB
    return;
#else
    int argcv = 2;
    int& argc = argcv;

    char *args[3];

    // We need to mimic the regular argv format to success with NPI init
    args[0]= (char*)"./presifuzz\0";
    args[1]= (char*)"-q\0";
    args[2]=NULL;
    char **p_args=args;
    char**& argv = p_args;

    npi_init(argc, argv);
#endif
  }

  npiCovHandle vdb_cov_init(const char* vdb_file_path) {
#ifdef DUMMY_LIB
    return 0;
#else
    npiCovHandle db = npi_cov_open( vdb_file_path );

    return db;
#endif
  }

  void vdb_cov_end(npiCovHandle db) {
#ifdef DUMMY_LIB
    return;
#else
    npi_cov_close( db );
    npi_end();
#endif
  }

  void instance_map_size( npiCovHandle scope, npiCovHandle test, CoverageMap* cov_map)
  {
#ifdef DUMMY_LIB
    return;
#else
    npiCovHandle inst_iter = npi_cov_iter_start( npiCovInstance, scope );
    npiCovHandle inst = NULL;
    while ( (inst = npi_cov_iter_next( inst_iter )) )
    {
      std::string cov_full_name = npi_cov_get_str( npiCovFullName, inst); 
      if( cov_full_name.rfind(cov_map->filter, 0) == 0  ) {
        // printf( "%s\n", npi_cov_get_str( npiCovFullName, inst ));
        compute_size( inst, test, cov_map);
      }
      
      instance_map_size( inst, test, cov_map);
    }
    npi_cov_iter_stop( inst_iter );
#endif
  }

  void compute_size( npiCovHandle inst, npiCovHandle test, CoverageMap* cov_map)
  {
#ifdef DUMMY_LIB
    return;
#else
    npiCovHandle metric = npi_cov_handle( (npiCovObjType_e)cov_map->type, inst );
    npiCovHandle iter = npi_cov_iter_start( npiCovChild, metric );
    npiCovHandle block;
    while ( (block = npi_cov_iter_next( iter )) )
    {
      int covered =  npi_cov_get( npiCovCovered, block, test );
      if(covered < 0)
        covered = 0;

      int coverable = npi_cov_get( npiCovCoverable, block, NULL );

      cov_map->coverable = cov_map->coverable + coverable;
      cov_map->covered = cov_map->covered + covered;
    }
    npi_cov_iter_stop( iter );
#endif
  }

  void dump_instance_coverage( npiCovHandle scope, npiCovHandle test, CoverageMap* cov_map)
  {
#ifdef DUMMY_LIB
    return;
#else
    npiCovHandle inst_iter = npi_cov_iter_start( npiCovInstance, scope );
    npiCovHandle inst = NULL;
    while ( (inst = npi_cov_iter_next( inst_iter )) )
    {
      std::string cov_full_name = npi_cov_get_str( npiCovFullName, inst); 
      if( cov_full_name.rfind(cov_map->filter, 0) == 0  ) {
        // printf( "%s\n", npi_cov_get_str( npiCovFullName, inst ));
        compute_score( inst, test, cov_map);
      }
      
      dump_instance_coverage( inst, test, cov_map);
    }
    npi_cov_iter_stop( inst_iter );
#endif
  }

  void compute_score( npiCovHandle inst, npiCovHandle test, CoverageMap* cov_map)
  {
#ifdef DUMMY_LIB
    return;
#else
    npiCovHandle metric = npi_cov_handle( (npiCovObjType_e)cov_map->type, inst );
    npiCovHandle iter = npi_cov_iter_start( npiCovChild, metric );
    npiCovHandle block;
    while ( (block = npi_cov_iter_next( iter )) )
    {
      int covered =  npi_cov_get( npiCovCovered, block, test );
      if(covered < 0)
        covered = 0;

      int coverable = npi_cov_get( npiCovCoverable, block, NULL );

      cov_map->coverable = cov_map->coverable + coverable;
      cov_map->covered = cov_map->covered + covered;

      for(int i=0; i< covered; i++) {
          cov_map->map[cov_map->write_byte_index] |= ((uint32_t)1 << cov_map->write_bit_index);
          cov_map->write_bit_index += 1;      

          if( cov_map->write_bit_index == 32 ) {
            cov_map->write_byte_index += 1;
            cov_map->write_bit_index = 0;      
          } 
      }
      
      for(int i=0; i< coverable-covered; i++) {
          cov_map->map[cov_map->write_byte_index] &= ~((uint32_t)1 << cov_map->write_bit_index);
          cov_map->write_bit_index += 1;      

          if( cov_map->write_bit_index == 32 ) {
            cov_map->write_byte_index += 1;
            cov_map->write_bit_index = 0;      
          } 
      }
    }
    npi_cov_iter_stop( iter );
#endif
  }

  void update_cov_map(npiCovHandle db, uint32_t* map, unsigned map_size, unsigned coverage_type, char* filter) {
#ifdef DUMMY_LIB
    std::srand(std::time(nullptr));

    int start = std::rand() % map_size;
    int end = ((std::rand() % map_size) + start) % map_size;

    for(int i=start; i<end; i++) {
        unsigned cov_dist = std::rand() % 100;
        // non uniform distribution
        // P=0.2 increases coverage
        // P=0.7 do nothing
        if( cov_dist < 20) {
            map[i] = std::rand() % 0xFF;
       }
    }

    return;
#else
    CoverageMap cov_map;
    cov_map.map = map;
    cov_map.write_byte_index = 2;
    cov_map.write_bit_index = 0;
    cov_map.type = coverage_type;
    cov_map.size = map_size;
    cov_map.coverable = 0;
    cov_map.covered = 0;
    cov_map.filter = filter;

    // printf("COVERAGE: %d\n", coverage_type);
    // printf("FILTER: %s\n", filter);

    // Iterate test and merge test
    npiCovHandle test_iter = npi_cov_iter_start( npiCovTest, db );
    npiCovHandle test;
    npiCovHandle merged_test = NULL;
    while ( (test = npi_cov_iter_next( test_iter) ) )
    {
      if ( merged_test == NULL )
        merged_test = test;
      else
      {
        merged_test = npi_cov_merge_test( merged_test, test );
        if ( merged_test == NULL )
        {
          return;
        }
      }
    }
    npi_cov_iter_stop( test_iter );

    // Dump instance requested type score from top
    dump_instance_coverage((void*)db, merged_test, &cov_map);

    npi_cov_close( db );
    npi_end();

    // float score = 0.0;
    // if(cov_map.coverable != 0) {
      // score = (((float)cov_map.covered / (float)cov_map.coverable) * 100.0);
    // }
    // printf("score is %f\n", score);
    // printf("coverable is %d\n", cov_map.coverable);
    // printf("covered is %d\n", cov_map.covered);
    // assumption: float is 4bytes length, fits in u32
    // map[0] = (uint32_t)score;
    map[0] = cov_map.covered;
    map[1] = cov_map.coverable;
#endif
  }

#ifdef __cplusplus
}
#endif

size_t compute_map_size(npiCovHandle db, unsigned coverage_type, char* filter) {
#ifdef DUMMY_LIB
    return 1024;
#else
    CoverageMap cov_map;
    cov_map.map = NULL;
    cov_map.write_byte_index = 2;
    cov_map.write_bit_index = 0;
    cov_map.type = coverage_type;
    cov_map.size = 1024;
    cov_map.coverable = 0;
    cov_map.covered = 0;
    cov_map.filter = filter;

    // Iterate test and merge test
    npiCovHandle test_iter = npi_cov_iter_start( npiCovTest, db );
    npiCovHandle test;
    npiCovHandle merged_test = NULL;
    while ( (test = npi_cov_iter_next( test_iter) ) )
    {
      if ( merged_test == NULL )
        merged_test = test;
      else
      {
        merged_test = npi_cov_merge_test( merged_test, test );
        if ( merged_test == NULL )
        {
          return 0;
        }
      }
    }
    npi_cov_iter_stop( test_iter );

    // Dump instance requested type score from top
    instance_map_size((void*)db, merged_test, &cov_map);

    npi_cov_close( db );
    npi_end();

    return cov_map.coverable;
#endif 
}

#ifdef C_APP
int main(int argc, char** argv) {

  vdb_init();

  void* db = vdb_cov_init(argv[1]);
  char* filter = "";

  unsigned metric = atoi(argv[2]);

  size_t size = compute_map_size(db, metric, filter);
  printf("Map size is %d for metric %d", size, metric);

  //uint32_t map[size] = {0};

  //update_cov_map(db, (uint32_t*)&map, size, 5, filter);

  //printf("[");
  //unsigned i;
  //for(i=0; i<size; i++) {
  //  printf("%u ", map[i]);
  //}
  //printf("]");
}
#endif
