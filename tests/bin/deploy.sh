#! /bin/sh
#
# pwd must be the git working directory when running this
#
# Change cmdprefix, $wwwpub, and $wwwhid to fit your web server configuration,
# the default works for an Ubuntu workstation with Apache.
#
# Requires -f or --force to reduce the risk that someone destroys their web
# server configuration by mistake.

if [ "-f" = "$1" ] || [ "--force" = "$1" ] ; then
    shift
else
    echo "requires -f or --force because it will destroy your website!"
    exit
fi
cmdprefix="sudo -u www-data"
wwwpub="/var/www/html/vicocomotest"
wwwhid="/var/www/vicocomotest"

for top in ${wwwpub} ${wwwhid} ; do
    if [ -d "${top}" ] ; then
      sudo rm -rf ${top}/*
    fi
    ${cmdprefix} mkdir -p ${top}
done

${cmdprefix} cp -r tests/public/* tests/public/.htaccess ${wwwpub}
for hid in config db f3 phpquery specs start.php templates vicocomotest ; do
    ${cmdprefix} cp -r tests/${hid} ${wwwhid}
done
${cmdprefix} mkdir -p ${wwwhid}/vicocomo
${cmdprefix} cp -r lib/* ${wwwhid}/vicocomo

