/*
    lang.c

    Copyright (C) 1997 Michael J. Fromberger, All Rights Reserved.

    Message strings for XCaliber Mark II
 */

/* Fixed messages -- these assume the con is called 'XYZ', for the nonce   */

#include "lang.h"

#include <stdio.h>
#include <stdlib.h>

#ifdef DEBUG
#include "mem.h"
#endif

#include "default.h"

#ifdef PATH_MAX
#define PATHLEN PATH_MAX
#else
#define PATHLEN DEFAULT_PATHLEN
#endif

#define LANGDIR_ENV "XCALLANGDIR"

char *lang_dir = DEFAULT_LANG_DIR;     /* from default.h */
char *lang_empty = DEFAULT_LANG_EMPTY; /* from default.h */

FILE *lang_open(char *fn, char *mode);

char *english_lang_str[] = {
    "Xcaliber is full--try again later\r\n",
    "\r\nConference \"XYZ\" has terminated.\r\n",
    "Hello.  Welcome to Xcaliber.\r\nPlease enter your name--",
    "Please enter your name--",
    "Hello.  You are the master terminal.\r\nPlease enter your name. ",
    "\r\nEnter command (type 'HELP' for instructions)\r\n"
    "Changes afoot!  Type EXPLAIN NEW for what's new!\r\n",
    "You are talking with #%d: %s\n\r",
    "\r\nNew user at #%d: %s\n\r",
    "\r\nDone\r\n",
    "\r\nMessage from #%d: %s\r\n",
    "\r\nMessage to all from #%d: %s\r\n",
    "\r\nNew name for #%d: %s\r\n",
    "\r\nEx-user at #%d: %s\r\n",
    "\r\nUser killed at #%d: %s\r\n",
    "\r\nSpeak!\r\n",
    "\r\nUser(s) out\r\n",
    "\r\nYou are now out...",
    "\r\nUser(s) ignoring you\r\n",
    "\r\nYou are no longer connected to conference \"XYZ\"\r\n",
    "\r\nNot sent--no recipients\r\n",
    "Message sent\r\n",
    "\r\nCommand error\r\n",
    "\r\nFormat error\r\n",
    "\r\nInvalid argument(s)\r\n",
    "\r\nCommand not implemented\r\n",
    "\r\nNew warning--",
    "\r\nNew name--",
    "\r\nNo one has left\r\n",
    "\r\nNo messages\r\n",
    "Message not sent\r\n",
    "\r\nEmpty message\r\n",
    "\r\nBad character\r\n",
    "\r\nTell-all not enabled\r\n",
    "\r\nCommand line--",
    "\r\nCan't port while rejecting\r\n",
    "\r\nRejecting ports\r\n",
    "Sorry--you've been bounced\r\n",
    "\r\nBounce list full\r\n",
    "\r\nNo bounced ports\r\n",
    "\r\nYou are now a master terminal\r\n",
    "\r\nYou are no longer a master terminal\r\n",
    "\r\nYou have been passed\r\n",
    "\r\nMust pass to a master terminal\r\n",
    "  New users\r\n",
    "\r\n\a\aYou have about %d %s left\r\n",
    "\r\nCan't explain that\r\n",
    "\r\nHelp-file not available\r\n",
    "%03d Users have left Xcaliber\r\n",
    " Left  Name\r\n",
    "\r\nXcaliber up at %2d:%02d:%02d %s\r\n",
    "\r\nUp at %d:%02d:%02d %s on %d/%d/%02d\r\n",
    "Time now: %d:%02d:%02d %s\r\n",
    "Time left:  %d minute(s)\r\n",
    "\r\n%d user(s)\r\n",
    " Users: %d\r\n   max: %d\r\nServer: %s v.%s",
    "\r\nXcaliber II v.%s by Michael J. Fromberger\r\n"
    "Copyright (C) 1997-1998 All Rights Reserved\r\n",
    "a.m.",
    "p.m.",
    "second",
    "seconds",
    "minute",
    "minutes",
    "\r\nNew language--",
    "\r\nLanguage not available\r\n",
    "\r\nCurrent language: %s\r\n",
    "\r\nCRUs now:  %3f\r\n     max:  %u\r\n",
    "\r\nCore size: %uK",
    "help",
    "[Yes]\r\n",
    "BREAK\r\n"};

char **lang_tab = english_lang_str;
int lang_len = num_lang_strs;

/*------------------------------------------------------------------------*/
char **lang_load(char *lng, int *ns) {
  char buf[80];
  char **out;
  int ix, len, num;
  FILE *fp;

  sprintf(buf, "%s.lx", lng);
  if ((fp = lang_open(buf, "rb")) == NULL) return NULL;

  fread(buf, sizeof(char), 2, fp);
  num = (buf[0] * 256) + buf[1];

  out = calloc(num_lang_strs, sizeof(char *));

  for (ix = 0; ix < num; ix++) {
    fread(buf, sizeof(char), 1, fp);
    len = buf[0];

    if (len > 0) {
      out[ix] = calloc(len + 1, sizeof(char));
      fread(out[ix], sizeof(char), len, fp);
      out[ix][len] = '\0';
    } else if (ix < num_lang_strs) {
      out[ix] = english_lang_str[ix];
    }
  }
  /* If any unfilled strings are left over, make sure
     they get the empty string so we don't bomb out
     trying to access through a null pointer later    */
  while (ix < num_lang_strs) {
    out[ix] = english_lang_str[ix];
    ++ix;
  }

  fclose(fp);

  if (ns) *ns = num;
  return out;
}

/*------------------------------------------------------------------------*/
void lang_free(char **ltab, int ns) {
  int ix;

  for (ix = 0; ix < ns; ix++) {
    if (ltab[ix] != NULL && ltab[ix] != lang_empty &&
        ltab[ix] != english_lang_str[ix])
      free(ltab[ix]);
    ltab[ix] = NULL;
  }
}

/*------------------------------------------------------------------------*/
FILE *lang_open(char *fn, char *mode) {
  char nbuf[PATHLEN];
  char *tmp;
  FILE *fp;

  if ((tmp = getenv(LANGDIR_ENV)) == NULL) tmp = lang_dir;

  sprintf(nbuf, "%s/%s", tmp, fn);
#ifdef DEBUG
  fprintf(stderr, "lang_open: opening file %s\n", nbuf);
#endif
  if ((fp = fopen(nbuf, mode)) == NULL) {
#ifdef DEBUG
    fprintf(stderr, "lang_open: failed\n");
#endif
    return NULL;
  } else {
#ifdef DEBUG
    fprintf(stderr, "lang_open: okay\n");
#endif
    return fp;
  }
}

/*------------------------------------------------------------------------*/
void lang_reset(void) {
  if (lang_tab != english_lang_str) {
    lang_free(lang_tab, lang_len);
  }

  lang_tab = english_lang_str;
  lang_len = num_lang_strs;
}

/*------------------------------------------------------------------------*/
int lang_install(char *lang) {
  char **strs;
  int nd;

  if ((strs = lang_load(lang, &nd)) == NULL) return 0;

  lang_reset();
  lang_tab = strs;
  lang_len = nd;
  return 1;
}
