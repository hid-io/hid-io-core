#!/bin/bash

# Structure for this script taken from
# https://github.com/steveklabnik/automatically_update_github_pages_with_travis_example

set -o errexit -o nounset

if [ "$TRAVIS_BRANCH" != "master" ]
then
  echo "This commit was made against the $TRAVIS_BRANCH and not the master! No deploy!"
  exit 0
fi

rev=$(git rev-parse --short HEAD)

mkdir -p target/doc
cd target/doc

git init
git config user.name "Jacob Alexander"
git config user.email "haata@kiibohd.com"

git remote add upstream "https://$GH_TOKEN@github.com/hid-io/hid-io.github.io"
git fetch upstream
git reset upstream/master

touch .

cargo doc
git add -A .
git commit -m "rebuild pages at ${rev}"
git push -q upstream HEAD:master

