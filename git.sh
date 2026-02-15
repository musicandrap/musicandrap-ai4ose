#!/bin/bash

# GitHub配置信息
USERNAME="name"        # 你的github用户名
TOKEN="token"          # 你的github的token令牌
REPO="musicandrap-ai4ose"  # 你的仓库名称

# 设置远程URL（带token）
git remote set-url origin https://${USERNAME}:${TOKEN}@github.com/${USERNAME}/${REPO}.git

echo "Finished"
