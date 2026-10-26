FROM node:24-alpine AS development
WORKDIR /app

ARG NPM_REGISTRY=https://registry.npmjs.org/
ENV NPM_CONFIG_REGISTRY=$NPM_REGISTRY

COPY package.json package-lock.json .npmrc ./
COPY docker/dev-start.sh /usr/local/bin/dev-start.sh
RUN chmod +x /usr/local/bin/dev-start.sh

EXPOSE 4210

CMD ["dev-start.sh"]
