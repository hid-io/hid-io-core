# Structure for this script taken from 
# https://github.com/steveklabnik/automatically_update_github_pages_with_travis_example

#!/bin/bash

set -o errexit -o nounset

if [ "$TRAVIS_BRANCH" != "master" ]
then
  echo "This commit was made against the $TRAVIS_BRANCH and not the master! No deploy!"
  exit 0
fi

rev=$(git rev-parse --short HEAD)

cd stage/_book

git init
git config user.name "Daniel Ho"
git config user.email "ho@berkeley.edu"

git remote add upstream "https://$GH_TOKEN@hid-io.github.io"
git fetch upstream
git reset upstream/gh-pages

touch .

cargo doc
git add -A .
git commit -m "rebuild pages at ${rev}"
git push -q upstream HEAD:gh-pages