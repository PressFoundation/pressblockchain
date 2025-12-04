import { ethers } from "hardhat";

async function main() {
  const [deployer] = await ethers.getSigners();
  console.log("Deploying with:", deployer.address);

  const PRESS = await ethers.getContractFactory("PRESS");
  const press = await PRESS.deploy(deployer.address);
  await press.waitForDeployment();
  console.log("PRESS:", await press.getAddress());

  const PressID = await ethers.getContractFactory("PressID");
  const pressId = await PressID.deploy(deployer.address);
  await pressId.waitForDeployment();
  console.log("PressID:", await pressId.getAddress());

  const OutletRegistry = await ethers.getContractFactory("OutletRegistry");
  const outletRegistry = await OutletRegistry.deploy(deployer.address);
  await outletRegistry.waitForDeployment();
  console.log("OutletRegistry:", await outletRegistry.getAddress());

  const ArticleNFT = await ethers.getContractFactory("ArticleNFT");
  const articleNFT = await ArticleNFT.deploy(
    deployer.address,
    await outletRegistry.getAddress()
  );
  await articleNFT.waitForDeployment();
  console.log("ArticleNFT:", await articleNFT.getAddress());

  const CourtModule = await ethers.getContractFactory("CourtModule");
  const court = await CourtModule.deploy(
    deployer.address,
    await pressId.getAddress()
  );
  await court.waitForDeployment();
  console.log("CourtModule:", await court.getAddress());

  const PressCouncil = await ethers.getContractFactory("PressCouncil");
  const council = await PressCouncil.deploy(
    deployer.address,
    await pressId.getAddress(),
    50,
    90 * 24 * 60 * 60
  );
  await council.waitForDeployment();
  console.log("PressCouncil:", await council.getAddress());

  const RoleManager = await ethers.getContractFactory("RoleManager");
  const roleManager = await RoleManager.deploy(
    deployer.address,
    await press.getAddress(),
    await pressId.getAddress(),
    await outletRegistry.getAddress(),
    await articleNFT.getAddress(),
    await court.getAddress(),
    await council.getAddress()
  );
  await roleManager.waitForDeployment();
  console.log("RoleManager:", await roleManager.getAddress());
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
