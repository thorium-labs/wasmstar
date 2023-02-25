import { instantiate } from "./instantiate";
import { upload } from "./upload"

const deploy = async () => {
    const codeId = await upload();
    await instantiate(String(codeId));
}

deploy();