/*
    log.c

    Copyright (C) 1997 Michael J. Fromberger, All Rights Reserved.

    Connexion logging for Xcaliber Mark II
 */

#include "log.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <netinet/in.h>

#include "default.h"

#ifdef PATH_MAX
#define PATHLEN PATH_MAX
#else
#define PATHLEN DEFAULT_PATHLEN
#endif

#define LOGDIR_ENV "XCALLOGDIR"

char *log_dir = DEFAULT_LOG_DIR;       /* from default.h */
int log_enabled = DEFAULT_LOG_ENABLED; /* from default.h */

FILE *log_open(char *fn, char *mode);

void log_connect(int ip, short port, short id) {
  FILE *fp;
  time_t now = time(NULL);

  if (!log_enabled) return;

  if ((fp = log_open("conlog", "a")) == NULL) return;

  fprintf(fp, "connect:%lu:%d:%d.%d.%d.%d:%u\n", (unsigned long)now, id,
          (ip >> 24) & 0xFF, (ip >> 16) & 0xFF, (ip >> 8) & 0xFF, ip & 0xFF,
          ntohs(port));
  fclose(fp);
}

void log_disconnect(short id) {
  FILE *fp;
  time_t now = time(NULL);

  if (!log_enabled) return;

  if ((fp = log_open("conlog", "a")) == NULL) return;

  fprintf(fp, "disconnect:%lu:%d\n", (unsigned long)now, id);
  fclose(fp);
}

void log_kill(short id, short killer) {
  FILE *fp;
  time_t now = time(NULL);

  if (!log_enabled) return;

  if ((fp = log_open("conlog", "a")) == NULL) return;

  fprintf(fp, "kill:%lu:%d:%d\n", (unsigned long)now, id, killer);
  fclose(fp);
}

void log_startup(int up) {
  FILE *fp;
  time_t now = time(NULL);

  if (!log_enabled) return;

  if ((fp = log_open("conlog", "a")) == NULL) return;

  if (up)
    fprintf(fp, "startup:%lu\n", (unsigned long)now);
  else
    fprintf(fp, "shutdown:%lu\n", (unsigned long)now);

  fclose(fp);
}

/*------------------------------------------------------------------------*/
FILE *log_open(char *fn, char *mode) {
  char nbuf[PATHLEN];
  char *tmp;
  FILE *fp;

  if ((tmp = getenv(LOGDIR_ENV)) == NULL) tmp = log_dir;

  sprintf(nbuf, "%s/%s", tmp, fn);
#ifdef DEBUG
  fprintf(stderr, "log_open: opening file %s\n", nbuf);
#endif
  if ((fp = fopen(nbuf, mode)) == NULL) {
#ifdef DEBUG
    fprintf(stderr, "log_open: failed\n");
#endif
    return NULL;
  } else {
#ifdef DEBUG
    fprintf(stderr, "log_open: okay\n");
#endif
    return fp;
  }
}
