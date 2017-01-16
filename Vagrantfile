# -*- mode: ruby -*-
# vi: set ft=ruby :

# All Vagrant configuration is done below. The "2" in Vagrant.configure
# configures the configuration version (we support older styles for
# backwards compatibility). Please don't change it unless you know what
# you're doing.
Vagrant.configure("2") do |config|
  config.vm.provider "virtualbox" do |vb|
    vb.memory = "2048"
    vb.cpus = 2
  end



  config.vm.define "old-debian" do |box|
    box.vm.box = "debian/wheezy64"

    box.vm.provision "shell", inline: <<-SHELL
      apt-get update
      apt-get install -y curl build-essential
      sudo -iu vagrant bash -c 'curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y'
    SHELL

  end

  config.vm.define "ubuntu-1610-32bit" do |box|
    box.vm.box = "box-cutter/ubuntu1610-i386"
    box.vm.provision "shell", inline: <<-SHELL
      apt-get update
      apt-get install -y curl build-essential
      sudo -iu vagrant bash -c 'curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y'
    SHELL

  end

  config.vm.define "ubuntu-1610-64bit" do |box|
    box.vm.box = "box-cutter/ubuntu1610"
    box.vm.provision "shell", inline: <<-SHELL
      apt-get update
      apt-get install -y curl build-essential
      sudo -iu vagrant bash -c 'curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y'
    SHELL

  end

  # This base box is buggy; you need to run `vagrant up freebsd-10-64` *twice*. The first time
  # it does with some networking configuration errors. Shared folders also don't work.
  # Bringing it up will hit issues near the end, but `vagrant ssh freebsd-10-64` does get you in.
  # Unfortunately that means it can't be auto provisioned. Run `vagrant provision` once the box is up.
  # Outside the vm, you can scp things in:
  # vagrant ssh-config freebsd-11 > freebsd-11.ssh_config
  # scp -F freebsd-11.ssh_config -r . freebsd-11:rust-auxv
  # Inside the vm:
  # bash -l
  # cd rust-auxv && cargo test
  config.vm.define "freebsd-11" do |box|
    box.ssh.shell = "/bin/csh"
    box.vm.box = "freebsd/FreeBSD-11.0-RELEASE-p1"
    box.vm.provision "shell", inline: <<-SHELL
      pkg install -y curl bash git
      su -l vagrant -c 'curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y'
    SHELL
  end

  # Couldn't get this to provision. Internet strangers claim these plugins are needed but it didn't help me
  # vagrant install plugin winrm
  # vagrant install plugin winrm-fs
  # Installing only the VS build tools didn't work for me. I needed to install VS Community (C++ lang selected)
  config.vm.define "windows-10" do |box|
    box.vm.box = "Microsoft/EdgeOnWindows10"
    box.vm.communicator = "winrm"
    box.vm.provider "virtualbox" do |vb|
      vb.memory = 4096
      vb.gui = true
    end
    box.vm.provision "shell", inline: <<-SHELL
      curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y
    SHELL

  end

  # Once the box starts, download xcode.
  # vagrant ssh-config macos > macos.ssh_config
  # scp -F macos.ssh_config -r . macos:rust-auxv
  config.vm.define "macos" do |box|
    box.vm.box = "jhcook/macos-sierra"
    box.vm.provider "virtualbox" do |vb|
      vb.memory = 4096
      vb.gui = true
    end
    box.vm.provision "shell", inline: <<-SHELL
      su -l vagrant -c 'curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y'
    SHELL

  end
end
